//! Inverse Kinematics (IK) and Forward Kinematics (FK) solvers.
//!
//! - [`FKSolver`]: computes world-space transforms from local joint poses.
//! - [`IKChain`]: defines a chain of joints for IK solving.
//! - [`IKTarget`]: target position (and optional rotation) for IK.
//! - [`IKSolver`]: CCD-based IK solver.
//! - [`ik_solve_system`]: ECS system entry point.

use engine_math::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::skeleton::{JointTransform, SkeletalAnimationPlayer, Skeleton};

use engine_ecs::world::World;

// ── Forward Kinematics ─────────────────────────────────────────────

/// Forward kinematics solver that computes world-space transforms by
/// traversing the joint hierarchy from root to leaves.
pub struct FKSolver;

impl FKSolver {
    /// Compute world-space transforms for all joints from a local pose.
    ///
    /// Traverses joints in parent-first order so that each parent's world
    /// transform is available before its children.
    pub fn compute_world_transforms(
        skeleton: &Skeleton,
        local_pose: &[JointTransform],
    ) -> Vec<Mat4> {
        let joint_count = skeleton.joint_count();
        let mut world = vec![Mat4::IDENTITY; joint_count];

        for i in 0..joint_count {
            let local = local_pose[i].to_matrix();
            world[i] = match skeleton.joints()[i].parent_index {
                Some(parent) => world[parent] * local,
                None => local,
            };
        }

        world
    }

    /// Compute the world-space transform for a single joint.
    pub fn compute_world_transform(
        skeleton: &Skeleton,
        local_pose: &[JointTransform],
        joint_index: usize,
    ) -> Mat4 {
        let chain = skeleton.parent_chain(joint_index);
        let mut world = Mat4::IDENTITY;

        // chain is [joint, parent, grandparent, ..., root], so iterate in reverse.
        for &idx in chain.iter().rev() {
            world = local_pose[idx].to_matrix() * world;
        }

        world
    }
}

// ── IK Components ──────────────────────────────────────────────────

/// Defines a chain of joints for IK solving.
///
/// Joints are ordered from **root to effector** (last = end-effector).
/// The solver adjusts rotations of all joints in the chain to reach the target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IKChain {
    /// Joint indices from root to effector (last = end-effector).
    pub joint_indices: Vec<usize>,
    /// Maximum solver iterations per frame.
    pub iterations: usize,
    /// Convergence tolerance in world-space units.
    pub tolerance: f32,
}

impl IKChain {
    /// Create a new IK chain from a list of joint names, resolving via the skeleton.
    ///
    /// Returns `None` if any joint name is not found.
    pub fn from_names(
        skeleton: &Skeleton,
        names: &[&str],
        iterations: usize,
        tolerance: f32,
    ) -> Option<Self> {
        let indices: Option<Vec<usize>> =
            names.iter().map(|name| skeleton.find_joint(name)).collect();
        indices.map(|joint_indices| Self {
            joint_indices,
            iterations,
            tolerance,
        })
    }

    /// Index of the end-effector (last joint in the chain).
    ///
    /// # Panics
    /// Panics if the chain is empty (invariant: chains must have ≥ 1 joint).
    pub fn effector_index(&self) -> usize {
        *self
            .joint_indices
            .last()
            .expect("IKChain must not be empty: invariant violation")
    }
}

/// Target for IK solving in world space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct IKTarget {
    /// World-space position the effector should reach.
    pub position: Vec3,
    /// Optional desired rotation for the effector.
    pub rotation: Option<Quat>,
}

impl IKTarget {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: None,
        }
    }

    pub fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = Some(rotation);
        self
    }
}

// ── CCD IK Solver ──────────────────────────────────────────────────

/// Inverse kinematics solver using the Cyclic Coordinate Descent (CCD) algorithm.
///
/// CCD iterates from the end-effector back to the root, rotating each joint
/// to bring the effector closer to the target. It is simple, robust, and
/// handles joint chains of arbitrary length.
pub struct IKSolver;

impl IKSolver {
    /// Solve IK, modifying `local_pose` in place.
    ///
    /// Returns `true` if the effector reached the target within tolerance.
    pub fn solve(
        skeleton: &Skeleton,
        chain: &IKChain,
        target: &IKTarget,
        local_pose: &mut [JointTransform],
    ) -> bool {
        if chain.joint_indices.len() < 2 {
            return false;
        }

        let effector_idx = chain.effector_index();

        for _ in 0..chain.iterations {
            // Check convergence.
            let world = FKSolver::compute_world_transforms(skeleton, local_pose);
            let effector_pos = world[effector_idx].transform_point3(Vec3::ZERO);
            if effector_pos.distance(target.position) <= chain.tolerance {
                return true;
            }

            // Iterate from the joint before the effector back to the root.
            for &joint_idx in chain.joint_indices.iter().rev().skip(1) {
                // Recompute world transforms after previous joint adjustments.
                let world = FKSolver::compute_world_transforms(skeleton, local_pose);
                let joint_pos = world[joint_idx].transform_point3(Vec3::ZERO);
                let current_effector = world[effector_idx].transform_point3(Vec3::ZERO);

                let to_effector = current_effector - joint_pos;
                let to_target = target.position - joint_pos;

                let len_effector = to_effector.length();
                let len_target = to_target.length();

                if len_effector < 1e-10 || len_target < 1e-10 {
                    continue;
                }

                let from_dir = to_effector / len_effector;
                let to_dir = to_target / len_target;

                // Compute rotation from current effector direction to target direction.
                let world_rotation = rotation_between(from_dir, to_dir);

                if world_rotation.length_squared() < 1e-10 {
                    continue;
                }

                // Convert world-space rotation to local-space for this joint.
                let parent_world = match skeleton.joints()[joint_idx].parent_index {
                    Some(parent) => world[parent],
                    None => Mat4::IDENTITY,
                };
                let parent_rotation = parent_world.to_scale_rotation_translation().1;

                let local_delta = parent_rotation.inverse() * world_rotation * parent_rotation;
                local_pose[joint_idx].rotation =
                    (local_delta * local_pose[joint_idx].rotation).normalize();
            }
        }

        // Final convergence check.
        let world = FKSolver::compute_world_transforms(skeleton, local_pose);
        let effector_pos = world[effector_idx].transform_point3(Vec3::ZERO);
        effector_pos.distance(target.position) <= chain.tolerance
    }

    /// Convenience: solve IK directly on a `SkeletalAnimationPlayer`.
    ///
    /// Returns `true` if converged.
    pub fn solve_with_player(
        skeleton: &Skeleton,
        chain: &IKChain,
        target: &IKTarget,
        player: &mut SkeletalAnimationPlayer,
    ) -> bool {
        Self::solve(skeleton, chain, target, player.local_pose_mut())
    }
}

/// Compute a quaternion rotation from `from` to `to` (both unit vectors).
fn rotation_between(from: Vec3, to: Vec3) -> Quat {
    let dot = from.dot(to).clamp(-1.0, 1.0);

    // Vectors are nearly opposite (180° apart).
    if dot < -0.999999 {
        // Find an orthogonal axis.
        let ortho = if from.x.abs() > 0.9 {
            from.cross(Vec3::Y)
        } else {
            from.cross(Vec3::X)
        }
        .normalize();
        return Quat::from_axis_angle(ortho, std::f32::consts::PI);
    }

    let cross = from.cross(to);
    Quat::from_xyzw(cross.x, cross.y, cross.z, 1.0 + dot).normalize()
}

// ── ECS System ─────────────────────────────────────────────────────

/// ECS system that runs IK solving.
///
/// For each entity that has an [`IKChain`], [`IKTarget`], [`Skeleton`], and
/// [`SkeletalAnimationPlayer`], applies CCD IK to move the effector toward
/// the target.
pub fn ik_solve_system(world: &mut World) {
    let entities: Vec<u32> = world.component_entities::<IKChain>();

    for entity_idx in entities {
        // Clone chain and target to release borrows before mutating player.
        let (chain, target) = {
            let chain = world.get_by_index::<IKChain>(entity_idx);
            let target = world.get_by_index::<IKTarget>(entity_idx);
            match (chain, target) {
                (Some(c), Some(t)) => (c.clone(), *t),
                _ => continue,
            }
        };

        let has_skeleton = world.get_by_index::<Skeleton>(entity_idx).is_some();
        let has_player = world
            .get_by_index::<SkeletalAnimationPlayer>(entity_idx)
            .is_some();

        if !has_skeleton || !has_player {
            continue;
        }

        // Clone skeleton to avoid borrow conflicts.
        let Some(skeleton) = world.get_by_index::<Skeleton>(entity_idx).cloned() else {
            continue;
        };
        if let Some(player) = world.get_by_index_mut::<SkeletalAnimationPlayer>(entity_idx) {
            IKSolver::solve_with_player(&skeleton, &chain, &target, player);
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skeleton::Joint;

    const EPS: f32 = 1e-2;

    /// Build a 3-joint chain along the X axis:
    /// root(0,0,0) → mid(10,0,0) → end(20,0,0)
    fn three_joint_skeleton() -> Skeleton {
        Skeleton::new(vec![
            Joint {
                id: 0,
                name: "root".to_string(),
                parent_index: None,
                inverse_bind_pose: Mat4::IDENTITY,
                local_bind_pose: Mat4::IDENTITY,
            },
            Joint {
                id: 1,
                name: "mid".to_string(),
                parent_index: Some(0),
                inverse_bind_pose: Mat4::from_translation(Vec3::new(-10.0, 0.0, 0.0)),
                local_bind_pose: Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            },
            Joint {
                id: 2,
                name: "end".to_string(),
                parent_index: Some(1),
                inverse_bind_pose: Mat4::from_translation(Vec3::new(-20.0, 0.0, 0.0)),
                local_bind_pose: Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)),
            },
        ])
    }

    #[test]
    fn test_fk_solver_world_transforms() {
        let skeleton = three_joint_skeleton();
        let local_pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();

        let world = FKSolver::compute_world_transforms(&skeleton, &local_pose);

        // Root at origin.
        let root_pos = world[0].transform_point3(Vec3::ZERO);
        assert!((root_pos - Vec3::ZERO).length() < EPS);

        // Mid at (10, 0, 0).
        let mid_pos = world[1].transform_point3(Vec3::ZERO);
        assert!((mid_pos - Vec3::new(10.0, 0.0, 0.0)).length() < EPS);

        // End at (20, 0, 0).
        let end_pos = world[2].transform_point3(Vec3::ZERO);
        assert!((end_pos - Vec3::new(20.0, 0.0, 0.0)).length() < EPS);
    }

    #[test]
    fn test_fk_solver_single_joint() {
        let skeleton = three_joint_skeleton();
        let local_pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();

        let end_world = FKSolver::compute_world_transform(&skeleton, &local_pose, 2);
        let end_pos = end_world.transform_point3(Vec3::ZERO);
        assert!((end_pos - Vec3::new(20.0, 0.0, 0.0)).length() < EPS);
    }

    #[test]
    fn test_fk_with_rotation() {
        let skeleton = three_joint_skeleton();
        let mut local_pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();

        // Rotate root 90° around Y → child should move along Z axis.
        local_pose[0].rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);

        let world = FKSolver::compute_world_transforms(&skeleton, &local_pose);

        let mid_pos = world[1].transform_point3(Vec3::ZERO);
        // After 90° Y rotation, (10,0,0) → (0,0,-10).
        assert!((mid_pos.x).abs() < EPS);
        assert!((mid_pos.z - (-10.0)).abs() < EPS);
    }

    #[test]
    fn test_ik_chain_from_names() {
        let skeleton = three_joint_skeleton();
        let chain = IKChain::from_names(&skeleton, &["root", "mid", "end"], 10, 0.01);
        assert!(chain.is_some());
        let chain = chain.unwrap();
        assert_eq!(chain.joint_indices, vec![0, 1, 2]);
        assert_eq!(chain.effector_index(), 2);

        // Missing joint name.
        let bad = IKChain::from_names(&skeleton, &["root", "missing"], 10, 0.01);
        assert!(bad.is_none());
    }

    #[test]
    fn test_ik_converges_to_target() {
        let skeleton = three_joint_skeleton();
        let chain = IKChain::from_names(&skeleton, &["root", "mid", "end"], 50, 0.1).unwrap();

        // Target: move effector from (20,0,0) to (15, 10, 0).
        let target = IKTarget::new(Vec3::new(15.0, 10.0, 0.0));

        let mut local_pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();

        let converged = IKSolver::solve(&skeleton, &chain, &target, &mut local_pose);
        assert!(converged, "IK should converge to target");

        // Verify effector is near target.
        let world = FKSolver::compute_world_transforms(&skeleton, &local_pose);
        let effector_pos = world[2].transform_point3(Vec3::ZERO);
        assert!(
            effector_pos.distance(target.position) < 0.5,
            "Effector {:?} should be near target {:?}",
            effector_pos,
            target.position
        );
    }

    #[test]
    fn test_ik_target_already_reached() {
        let skeleton = three_joint_skeleton();
        let chain = IKChain::from_names(&skeleton, &["root", "mid", "end"], 10, 0.1).unwrap();

        // Target at current effector position.
        let target = IKTarget::new(Vec3::new(20.0, 0.0, 0.0));

        let mut local_pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();

        let converged = IKSolver::solve(&skeleton, &chain, &target, &mut local_pose);
        assert!(
            converged,
            "Should converge immediately when target is at effector"
        );
    }

    #[test]
    fn test_ik_with_player() {
        let skeleton = three_joint_skeleton();
        let chain = IKChain::from_names(&skeleton, &["root", "mid", "end"], 50, 0.1).unwrap();
        let target = IKTarget::new(Vec3::new(10.0, 15.0, 0.0));

        let mut player = SkeletalAnimationPlayer::new("test", &skeleton);

        let converged = IKSolver::solve_with_player(&skeleton, &chain, &target, &mut player);
        assert!(converged);

        let world = FKSolver::compute_world_transforms(&skeleton, player.local_pose());
        let effector_pos = world[2].transform_point3(Vec3::ZERO);
        assert!(effector_pos.distance(target.position) < 0.5);
    }

    #[test]
    fn test_ik_chain_too_short() {
        let skeleton = three_joint_skeleton();
        // Chain with only 1 joint — cannot solve.
        let chain = IKChain {
            joint_indices: vec![0],
            iterations: 10,
            tolerance: 0.1,
        };
        let target = IKTarget::new(Vec3::new(100.0, 0.0, 0.0));
        let mut local_pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();

        let converged = IKSolver::solve(&skeleton, &chain, &target, &mut local_pose);
        assert!(!converged);
    }

    #[test]
    fn test_ik_target_behind_root() {
        let skeleton = three_joint_skeleton();
        let chain = IKChain::from_names(&skeleton, &["root", "mid", "end"], 100, 0.1).unwrap();

        // Target behind the root — chain must bend back.
        let target = IKTarget::new(Vec3::new(-10.0, 5.0, 0.0));

        let mut local_pose: Vec<JointTransform> = skeleton
            .joints()
            .iter()
            .map(|j| JointTransform::from_matrix(j.local_bind_pose))
            .collect();

        let converged = IKSolver::solve(&skeleton, &chain, &target, &mut local_pose);
        assert!(
            converged,
            "IK should converge even when target is behind root"
        );

        let world = FKSolver::compute_world_transforms(&skeleton, &local_pose);
        let effector_pos = world[2].transform_point3(Vec3::ZERO);
        assert!(effector_pos.distance(target.position) < 1.0);
    }

    #[test]
    fn test_rotation_between() {
        let from = Vec3::X;
        let to = Vec3::Y;
        let q = rotation_between(from, to);
        let rotated = q * from;
        assert!((rotated - to).length() < EPS);
    }

    #[test]
    fn test_rotation_between_opposite() {
        let from = Vec3::X;
        let to = -Vec3::X;
        let q = rotation_between(from, to);
        let rotated = q * from;
        assert!((rotated - to).length() < EPS);
    }
}
