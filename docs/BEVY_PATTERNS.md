# Bevy Best Practices & Patterns

## Component Spawning Patterns

### 1. **Bundles for Complex Entities** (Current Pattern)
**Use when:** You have a complex entity with many components that needs asset creation

```rust
#[derive(Bundle)]
pub struct ProbeCameraBundle {
    pub probe_camera: ProbeCamera,
    pub transform: Transform,
    pub camera: Camera,
    // ... many components
}

// Helper function that creates assets and returns bundle
pub fn spawn_probe_camera(...) -> (Entity, Vec2) {
    // Create assets
    let image_handle = images.add(image);
    // Build bundle
    let bundle = ProbeCameraBundle { ... };
    commands.spawn(bundle).id()
}
```

**Pros:**
- Type-safe grouping
- Reusable across codebase
- Clear what components are included
- Works well with asset creation helpers

**Cons:**
- Requires defining a struct
- More verbose for simple cases

---

### 2. **Tuples for Simple Entities** (Modern Alternative)
**Use when:** Simple entity, no asset creation needed

```rust
commands.spawn((
    ProbeCamera { resolution },
    Transform::from_xyz(0.0, 0.0, 0.0),
    Camera::default(),
    Camera3d::default(),
));
```

**Pros:**
- No struct definition needed
- Inline and clear
- Less boilerplate

**Cons:**
- Not reusable
- Harder to maintain when component list grows
- Doesn't work well with asset creation patterns

---

### 3. **`#[require(...)]` for Invariants** (Validation Pattern)
**Use when:** You want to enforce that a component MUST have certain other components

```rust
#[derive(Component)]
#[require(Camera, Camera3d, Transform)]
pub struct ProbeCamera {
    pub resolution: UVec2,
}
```

**What it does:**
- **Validates** at runtime that entities with `ProbeCamera` also have the required components
- **Does NOT spawn** the required components automatically
- Helps catch bugs in queries
- Makes component relationships explicit

**You still need to spawn the components:**
```rust
// Still need to spawn all components together
commands.spawn((
    ProbeCamera { resolution },
    Camera::default(),  // Required
    Camera3d::default(), // Required
    Transform::default(), // Required
));
```

---

### 4. **Hybrid: Bundle + `#[require(...)]`** (Best of Both Worlds)
**Use when:** You want both reusable spawning AND enforced invariants

```rust
// Enforce the invariant
#[derive(Component)]
#[require(Camera, Camera3d, Transform, KernelBindGroup)]
pub struct ProbeCamera {
    pub resolution: UVec2,
}

// Provide convenient spawning
#[derive(Bundle)]
pub struct ProbeCameraBundle {
    pub probe_camera: ProbeCamera,
    pub transform: Transform,
    pub camera: Camera,
    pub camera_3d: Camera3d,
    pub kernel_bind_group: KernelBindGroup,
    // ... other components
}
```

---

## Decision Tree

```
Need asset creation or complex setup?
├─ YES → Use Bundle + helper function (current pattern)
└─ NO → Simple entity?
    ├─ YES → Use tuple spawn
    └─ NO (many components) → Use Bundle

Want to enforce invariants?
└─ YES → Add #[require(...)] to your marker component
```

---

## Recommendation for Probe Camera

**Current approach (Bundle + helper function) is correct** because:
1. ✅ Needs asset creation (Image, ShaderStorageBuffer)
2. ✅ Complex setup with many components
3. ✅ Reusable across codebase
4. ✅ Type-safe and maintainable

**Optional enhancement:** Add `#[require(...)]` to `ProbeCamera` to enforce invariants:

```rust
#[derive(Component)]
#[require(Camera, Camera3d, Transform, KernelBindGroup)]
pub struct ProbeCamera {
    pub resolution: UVec2,
}
```

This ensures that if someone manually adds `ProbeCamera` to an entity, they must also add the required components (validation), but doesn't change how we spawn entities (still use the Bundle).

---

## Trait Extensions (Alternative Pattern)

You can also add convenience methods via trait extensions:

```rust
pub trait ProbeCameraCommands {
    fn spawn_probe_camera(
        &mut self,
        transform: Transform,
        resolution: UVec2,
        images: &mut Assets<Image>,
        ssbo_assets: &mut Assets<ShaderStorageBuffer>,
    ) -> Entity;
}

impl ProbeCameraCommands for Commands<'_, '_> {
    fn spawn_probe_camera(...) -> Entity {
        // Implementation
    }
}

// Usage:
commands.spawn_probe_camera(transform, resolution, &mut images, &mut ssbo_assets);
```

**When to use:**
- API convenience methods
- Domain-specific spawning patterns
- When you want method chaining: `commands.spawn_probe_camera(...).insert(...)`

**Trade-off:**
- More setup (trait + impl)
- Convenient usage
- Works well alongside Bundles
