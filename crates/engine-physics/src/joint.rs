use engine_math::Vec3;

/// Type of physics joint constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointType {
    /// Ball-and-socket joint: allows rotation in all directions.
    /// Constrained by a maximum angle from the rest direction.
    BallSocket,
    /// Hinge joint: allows rotation around a single axis.
    /// Constrained by min/max angle limits.
    Hinge,
    /// Spring joint: applies force to maintain a target distance.
    Spring,
}

/// A physics joint connecting two bodies.
///
/// Joints constrain the relative motion of two rigid bodies.
/// `entity_a` and `entity_b` are ECS entity indices.
#[derive(Debug, Clone)]
pub struct Joint {
    pub joint_type: JointType,
    /// First body (entity index).
    pub entity_a: u32,
    /// Second body (entity index).
    pub entity_b: u32,
    /// Anchor point on body A (local space).
    pub anchor_a: Vec3,
    /// Anchor point on body B (local space).
    pub anchor_b: Vec3,
    /// Axis of rotation (local to body A). Used by hinge joints.
    pub axis: Vec3,
    /// Minimum angle limit in radians (hinge).
    pub min_angle: f32,
    /// Maximum angle limit in radians (hinge).
    pub max_angle: f32,
    /// Maximum cone angle in radians (ball-socket).
    pub max_cone_angle: f32,
    /// Spring stiffness (spring joint).
    pub stiffness: f32,
    /// Spring damping (spring joint).
    pub damping: f32,
    /// Rest length / distance (spring joint).
    pub rest_length: f32,
    /// Whether this joint is enabled.
    pub enabled: bool,
}

impl Joint {
    /// Create a ball-socket joint between two entities.
    pub fn ball_socket(entity_a: u32, entity_b: u32, anchor_a: Vec3, anchor_b: Vec3) -> Self {
        Self {
            joint_type: JointType::BallSocket,
            entity_a,
            entity_b,
            anchor_a,
            anchor_b,
            axis: Vec3::Y,
            min_angle: 0.0,
            max_angle: 0.0,
            max_cone_angle: std::f32::consts::FRAC_PI_4, // 45 degrees
            stiffness: 0.0,
            damping: 0.0,
            rest_length: 0.0,
            enabled: true,
        }
    }

    /// Create a hinge joint between two entities.
    pub fn hinge(entity_a: u32, entity_b: u32, anchor_a: Vec3, anchor_b: Vec3, axis: Vec3) -> Self {
        Self {
            joint_type: JointType::Hinge,
            entity_a,
            entity_b,
            anchor_a,
            anchor_b,
            axis: axis.normalize(),
            min_angle: -std::f32::consts::FRAC_PI_2,
            max_angle: std::f32::consts::FRAC_PI_2,
            max_cone_angle: 0.0,
            stiffness: 0.0,
            damping: 0.0,
            rest_length: 0.0,
            enabled: true,
        }
    }

    /// Create a spring joint between two entities.
    pub fn spring(
        entity_a: u32,
        entity_b: u32,
        anchor_a: Vec3,
        anchor_b: Vec3,
        rest_length: f32,
        stiffness: f32,
        damping: f32,
    ) -> Self {
        Self {
            joint_type: JointType::Spring,
            entity_a,
            entity_b,
            anchor_a,
            anchor_b,
            axis: Vec3::Y,
            min_angle: 0.0,
            max_angle: 0.0,
            max_cone_angle: 0.0,
            stiffness,
            damping,
            rest_length,
            enabled: true,
        }
    }

    /// Set angle limits for hinge joints (in radians).
    pub fn with_angle_limits(mut self, min: f32, max: f32) -> Self {
        self.min_angle = min;
        self.max_angle = max;
        self
    }

    /// Set the cone limit for ball-socket joints (in radians).
    pub fn with_cone_limit(mut self, max_angle: f32) -> Self {
        self.max_cone_angle = max_angle;
        self
    }
}

/// Constraint solver for joints.
///
/// Applies positional corrections to enforce joint constraints.
pub struct JointSolver {
    pub joints: Vec<Joint>,
    pub iterations: u32,
}

impl Default for JointSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl JointSolver {
    pub fn new() -> Self {
        Self {
            joints: Vec::new(),
            iterations: 10,
        }
    }

    /// Add a joint to the solver.
    pub fn add_joint(&mut self, joint: Joint) {
        self.joints.push(joint);
    }

    /// Remove a joint by index.
    pub fn remove_joint(&mut self, index: usize) -> Option<Joint> {
        if index < self.joints.len() {
            Some(self.joints.remove(index))
        } else {
            None
        }
    }

    /// Remove all joints involving a specific entity.
    pub fn remove_joints_for_entity(&mut self, entity: u32) {
        self.joints
            .retain(|j| j.entity_a != entity && j.entity_b != entity);
    }

    /// Get the number of active joints.
    pub fn joint_count(&self) -> usize {
        self.joints.iter().filter(|j| j.enabled).count()
    }

    /// Solve spring constraints.
    ///
    /// For each spring joint, compute the displacement from rest length
    /// and return force corrections for each entity.
    pub fn solve_springs(
        &self,
        positions: &[(u32, Vec3)],
        velocities: &[(u32, Vec3)],
    ) -> Vec<(u32, Vec3)> {
        let mut corrections: Vec<(u32, Vec3)> = Vec::new();

        for joint in &self.joints {
            if !joint.enabled || joint.joint_type != JointType::Spring {
                continue;
            }

            let pos_a = positions
                .iter()
                .find(|(e, _)| *e == joint.entity_a)
                .map(|(_, p)| *p);
            let pos_b = positions
                .iter()
                .find(|(e, _)| *e == joint.entity_b)
                .map(|(_, p)| *p);

            let (Some(pa), Some(pb)) = (pos_a, pos_b) else {
                continue;
            };

            let vel_a = velocities
                .iter()
                .find(|(e, _)| *e == joint.entity_a)
                .map(|(_, v)| *v)
                .unwrap_or(Vec3::ZERO);
            let vel_b = velocities
                .iter()
                .find(|(e, _)| *e == joint.entity_b)
                .map(|(_, v)| *v)
                .unwrap_or(Vec3::ZERO);

            let delta = pb - pa;
            let dist = delta.length();
            if dist < 1e-6 {
                continue;
            }

            let dir = delta / dist;
            let extension = dist - joint.rest_length;

            // Spring force: F = -k * x - d * v_relative
            let rel_vel = (vel_b - vel_a).dot(dir);
            let force_magnitude = -joint.stiffness * extension - joint.damping * rel_vel;
            let force = dir * force_magnitude;

            corrections.push((joint.entity_a, -force));
            corrections.push((joint.entity_b, force));
        }

        corrections
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ball_socket_joint() {
        let j = Joint::ball_socket(0, 1, Vec3::ZERO, Vec3::ZERO);
        assert_eq!(j.joint_type, JointType::BallSocket);
        assert_eq!(j.entity_a, 0);
        assert_eq!(j.entity_b, 1);
        assert!(j.enabled);
    }

    #[test]
    fn test_hinge_joint() {
        let j = Joint::hinge(0, 1, Vec3::ZERO, Vec3::ZERO, Vec3::Y);
        assert_eq!(j.joint_type, JointType::Hinge);
        assert!((j.min_angle + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn test_spring_joint() {
        let j = Joint::spring(0, 1, Vec3::ZERO, Vec3::ZERO, 5.0, 100.0, 10.0);
        assert_eq!(j.joint_type, JointType::Spring);
        assert!((j.rest_length - 5.0).abs() < 1e-6);
        assert!((j.stiffness - 100.0).abs() < 1e-6);
    }

    #[test]
    fn test_joint_builder_methods() {
        let j = Joint::hinge(0, 1, Vec3::ZERO, Vec3::ZERO, Vec3::Y).with_angle_limits(-1.0, 1.0);
        assert!((j.min_angle - (-1.0)).abs() < 1e-6);
        assert!((j.max_angle - 1.0).abs() < 1e-6);

        let j2 = Joint::ball_socket(0, 1, Vec3::ZERO, Vec3::ZERO).with_cone_limit(0.5);
        assert!((j2.max_cone_angle - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_joint_solver_add_remove() {
        let mut solver = JointSolver::new();
        solver.add_joint(Joint::ball_socket(0, 1, Vec3::ZERO, Vec3::ZERO));
        solver.add_joint(Joint::hinge(2, 3, Vec3::ZERO, Vec3::ZERO, Vec3::Y));
        assert_eq!(solver.joint_count(), 2);

        solver.remove_joint(0);
        assert_eq!(solver.joint_count(), 1);
    }

    #[test]
    fn test_joint_solver_remove_for_entity() {
        let mut solver = JointSolver::new();
        solver.add_joint(Joint::ball_socket(0, 1, Vec3::ZERO, Vec3::ZERO));
        solver.add_joint(Joint::hinge(0, 2, Vec3::ZERO, Vec3::ZERO, Vec3::Y));
        solver.add_joint(Joint::ball_socket(1, 2, Vec3::ZERO, Vec3::ZERO));

        solver.remove_joints_for_entity(0);
        assert_eq!(solver.joints.len(), 1);
        assert_eq!(solver.joints[0].entity_a, 1);
    }

    #[test]
    fn test_spring_force_computation() {
        let mut solver = JointSolver::new();
        solver.add_joint(Joint::spring(0, 1, Vec3::ZERO, Vec3::ZERO, 5.0, 100.0, 0.0));

        let positions = vec![(0, Vec3::ZERO), (1, Vec3::new(10.0, 0.0, 0.0))];
        let velocities = vec![(0, Vec3::ZERO), (1, Vec3::ZERO)];

        let corrections = solver.solve_springs(&positions, &velocities);
        assert_eq!(corrections.len(), 2);
        // Body 0 should be pushed toward body 1 (positive x)
        assert!(corrections[0].1.x > 0.0);
        // Body 1 should be pushed toward body 0 (negative x)
        assert!(corrections[1].1.x < 0.0);
    }

    #[test]
    fn test_disabled_joint_ignored() {
        let mut solver = JointSolver::new();
        let mut j = Joint::spring(0, 1, Vec3::ZERO, Vec3::ZERO, 5.0, 100.0, 0.0);
        j.enabled = false;
        solver.add_joint(j);

        let positions = vec![(0, Vec3::ZERO), (1, Vec3::new(10.0, 0.0, 0.0))];
        let velocities = vec![(0, Vec3::ZERO), (1, Vec3::ZERO)];

        let corrections = solver.solve_springs(&positions, &velocities);
        assert!(corrections.is_empty());
    }
}
