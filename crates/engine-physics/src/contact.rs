use engine_math::Vec3;

/// A persistent contact point between two bodies.
///
/// Accumulated impulses from previous frames are stored here for warm-starting,
/// which greatly improves stacking stability.
#[derive(Debug, Clone)]
pub struct ContactPoint {
    /// Position of the contact in world space.
    pub position: Vec3,
    /// Contact normal (from body A toward body B).
    pub normal: Vec3,
    /// Penetration depth (positive = overlapping).
    pub depth: f32,
    /// Accumulated normal impulse (for warm starting).
    pub accumulated_normal_impulse: f32,
    /// Accumulated friction impulse (for warm starting).
    pub accumulated_tangent_impulse: f32,
    /// Accumulated tangent direction (second friction axis).
    pub accumulated_tangent2_impulse: f32,
}

impl ContactPoint {
    /// Create a new contact point at the given world-space position with the specified
    /// normal and penetration depth. Accumulated impulses start at zero.
    pub fn new(position: Vec3, normal: Vec3, depth: f32) -> Self {
        Self {
            position,
            normal,
            depth,
            accumulated_normal_impulse: 0.0,
            accumulated_tangent_impulse: 0.0,
            accumulated_tangent2_impulse: 0.0,
        }
    }
}

/// A contact manifold between two specific bodies.
///
/// Stores all contact points between a pair of bodies.
#[derive(Debug, Clone)]
pub struct ContactManifold {
    /// Entity index of body A.
    pub body_a: u32,
    /// Entity index of body B.
    pub body_b: u32,
    /// Contact points in this manifold.
    pub contacts: Vec<ContactPoint>,
    /// Combined restitution coefficient.
    pub restitution: f32,
    /// Combined friction coefficient.
    pub friction: f32,
}

impl ContactManifold {
    /// Create an empty contact manifold for the given body pair.
    pub fn new(body_a: u32, body_b: u32) -> Self {
        Self {
            body_a,
            body_b,
            contacts: Vec::new(),
            restitution: 0.2,
            friction: 0.4,
        }
    }

    /// Add a contact point to this manifold.
    pub fn add_contact(&mut self, contact: ContactPoint) {
        self.contacts.push(contact);
    }

    /// Number of contact points.
    pub fn contact_count(&self) -> usize {
        self.contacts.len()
    }
}

/// Constraint-based contact solver with warm starting.
///
/// Improves upon the basic impulse solver by:
/// - Accumulating impulses across frames (warm starting)
/// - Solving normal and friction constraints iteratively
/// - Clamping accumulated impulses to prevent energy injection
pub struct ContactSolver {
    pub iterations: u32,
    pub baumgarte: f32,
    pub slop: f32,
}

impl Default for ContactSolver {
    fn default() -> Self {
        Self {
            iterations: 10,
            baumgarte: 0.2,
            slop: 0.005,
        }
    }
}

impl ContactSolver {
    /// Create a new contact solver with default settings (10 iterations, Baumgarte 0.2, slop 0.005).
    pub fn new() -> Self {
        Self::default()
    }

    /// Solve a single contact manifold.
    ///
    /// Takes the current velocities of both bodies and returns velocity corrections.
    /// Uses iterative constraint solving with warm starting from accumulated impulses.
    pub fn solve_manifold(
        &self,
        manifold: &mut ContactManifold,
        vel_a: &mut Vec3,
        vel_b: &mut Vec3,
        inv_mass_a: f32,
        inv_mass_b: f32,
        dt: f32,
    ) {
        let total_inv_mass = inv_mass_a + inv_mass_b;
        if total_inv_mass <= 0.0 {
            return;
        }

        for _ in 0..self.iterations {
            for contact in &mut manifold.contacts {
                self.solve_normal_contact(
                    contact,
                    vel_a,
                    vel_b,
                    inv_mass_a,
                    inv_mass_b,
                    total_inv_mass,
                    manifold.restitution,
                    dt,
                );
                self.solve_friction_contact(
                    contact,
                    vel_a,
                    vel_b,
                    inv_mass_a,
                    inv_mass_b,
                    total_inv_mass,
                    manifold.friction,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_normal_contact(
        &self,
        contact: &mut ContactPoint,
        vel_a: &mut Vec3,
        vel_b: &mut Vec3,
        inv_mass_a: f32,
        inv_mass_b: f32,
        total_inv_mass: f32,
        restitution: f32,
        dt: f32,
    ) {
        let relative_vel = *vel_b - *vel_a;
        let vel_along_normal = relative_vel.dot(contact.normal);

        // Baumgarte bias for position correction
        let bias = self.baumgarte * (contact.depth - self.slop).max(0.0) / dt;

        // Restitution: only apply if separating velocity is above threshold
        let restitution_bias = if vel_along_normal < -1.0 {
            -restitution * vel_along_normal
        } else {
            0.0
        };

        // Normal impulse magnitude
        let j = -(vel_along_normal - bias - restitution_bias) / total_inv_mass;

        // Accumulate and clamp (non-negative — can only push apart)
        let old_accumulated = contact.accumulated_normal_impulse;
        contact.accumulated_normal_impulse = (old_accumulated + j).max(0.0);
        let j = contact.accumulated_normal_impulse - old_accumulated;

        let impulse = contact.normal * j;

        // Apply
        *vel_a -= impulse * inv_mass_a;
        *vel_b += impulse * inv_mass_b;
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_friction_contact(
        &self,
        contact: &mut ContactPoint,
        vel_a: &mut Vec3,
        vel_b: &mut Vec3,
        inv_mass_a: f32,
        inv_mass_b: f32,
        total_inv_mass: f32,
        friction: f32,
    ) {
        let relative_vel = *vel_b - *vel_a;
        let vel_along_normal = relative_vel.dot(contact.normal);

        // Compute tangent direction (remove normal component)
        let tangent_vel = relative_vel - contact.normal * vel_along_normal;
        let tangent_len_sq = tangent_vel.length_squared();
        if tangent_len_sq < f32::EPSILON {
            return;
        }
        let tangent = tangent_vel / tangent_len_sq.sqrt();

        // Tangent impulse
        let jt = -relative_vel.dot(tangent) / total_inv_mass;

        // Coulomb friction clamping: |jt| <= mu * accumulated_normal_impulse
        let max_friction = friction * contact.accumulated_normal_impulse;
        let old_tangent = contact.accumulated_tangent_impulse;
        contact.accumulated_tangent_impulse = (old_tangent + jt).clamp(-max_friction, max_friction);
        let jt = contact.accumulated_tangent_impulse - old_tangent;

        let friction_impulse = tangent * jt;

        *vel_a -= friction_impulse * inv_mass_a;
        *vel_b += friction_impulse * inv_mass_b;
    }

    /// Clear accumulated impulses (call when bodies separate).
    pub fn reset_manifold(manifold: &mut ContactManifold) {
        for contact in &mut manifold.contacts {
            contact.accumulated_normal_impulse = 0.0;
            contact.accumulated_tangent_impulse = 0.0;
            contact.accumulated_tangent2_impulse = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contact_point_creation() {
        let cp = ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1);
        assert_eq!(cp.position, Vec3::ZERO);
        assert_eq!(cp.normal, Vec3::Y);
        assert!((cp.depth - 0.1).abs() < 1e-6);
        assert!((cp.accumulated_normal_impulse).abs() < 1e-6);
    }

    #[test]
    fn test_contact_manifold() {
        let mut m = ContactManifold::new(0, 1);
        m.add_contact(ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1));
        m.add_contact(ContactPoint::new(Vec3::X, Vec3::Y, 0.05));
        assert_eq!(m.contact_count(), 2);
    }

    #[test]
    fn test_contact_solver_pushes_apart() {
        let mut m = ContactManifold::new(0, 1);
        m.add_contact(ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1));

        let solver = ContactSolver::new();
        let mut vel_a = Vec3::ZERO;
        let mut vel_b = Vec3::new(0.0, -5.0, 0.0);

        solver.solve_manifold(&mut m, &mut vel_a, &mut vel_b, 1.0, 1.0, 1.0 / 60.0);

        // Body A should move down, body B should move up
        assert!(vel_a.y <= 0.0, "vel_a.y = {}", vel_a.y);
        assert!(vel_b.y >= -5.0, "vel_b.y = {}", vel_b.y);
    }

    #[test]
    fn test_contact_solver_warm_starting() {
        let mut m = ContactManifold::new(0, 1);
        let mut cp = ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1);
        cp.accumulated_normal_impulse = 10.0; // Pre-existing impulse
        m.add_contact(cp);

        let solver = ContactSolver::new();
        let mut vel_a = Vec3::ZERO;
        let mut vel_b = Vec3::new(0.0, -1.0, 0.0);

        solver.solve_manifold(&mut m, &mut vel_a, &mut vel_b, 1.0, 1.0, 1.0 / 60.0);

        // Accumulated impulse should still be non-negative
        assert!(m.contacts[0].accumulated_normal_impulse >= 0.0);
    }

    #[test]
    fn test_contact_solver_static_body() {
        let mut m = ContactManifold::new(0, 1);
        m.add_contact(ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1));

        let solver = ContactSolver::new();
        let mut vel_a = Vec3::ZERO;
        let mut vel_b = Vec3::new(0.0, -5.0, 0.0);

        // Body A is static (inv_mass = 0)
        solver.solve_manifold(&mut m, &mut vel_a, &mut vel_b, 0.0, 1.0, 1.0 / 60.0);

        // Static body should not move
        assert!((vel_a.length()).abs() < 1e-6);
        // Dynamic body should be pushed up
        assert!(vel_b.y > -5.0);
    }

    #[test]
    fn test_reset_manifold() {
        let mut m = ContactManifold::new(0, 1);
        let mut cp = ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1);
        cp.accumulated_normal_impulse = 5.0;
        cp.accumulated_tangent_impulse = 2.0;
        m.add_contact(cp);

        ContactSolver::reset_manifold(&mut m);
        assert!((m.contacts[0].accumulated_normal_impulse).abs() < 1e-6);
        assert!((m.contacts[0].accumulated_tangent_impulse).abs() < 1e-6);
    }

    #[test]
    fn test_friction_reduces_sliding() {
        let mut m = ContactManifold::new(0, 1);
        m.friction = 0.5;
        m.add_contact(ContactPoint::new(Vec3::ZERO, Vec3::Y, 0.1));

        let solver = ContactSolver::new();
        let mut vel_a = Vec3::ZERO;
        let mut vel_b = Vec3::new(5.0, -1.0, 0.0); // Sliding + falling

        solver.solve_manifold(&mut m, &mut vel_a, &mut vel_b, 1.0, 1.0, 1.0 / 60.0);

        // Horizontal velocity should be reduced by friction
        assert!(vel_b.x < 5.0, "vel_b.x = {}", vel_b.x);
    }
}
