# Command Extensions vs Functions

## Current Approach: Function

```rust
pub fn spawn_probe_camera(
    commands: &mut Commands,
    transform: Transform,
    resolution: UVec2,
    images: &mut Assets<Image>,
    ssbo_assets: &mut Assets<ShaderStorageBuffer>,
) -> (Entity, Vec2)

// Usage:
spawn_probe_camera(&mut commands, transform, resolution, &mut images, &mut ssbo_assets);
```

## Alternative: Trait Extension

```rust
pub trait ProbeCameraCommands {
    fn spawn_probe_camera(
        &mut self,
        transform: Transform,
        resolution: UVec2,
        images: &mut Assets<Image>,
        ssbo_assets: &mut Assets<ShaderStorageBuffer>,
    ) -> EntityCommands;
}

impl ProbeCameraCommands for Commands<'_, '_> {
    fn spawn_probe_camera(...) -> EntityCommands {
        // Implementation
    }
}

// Usage:
commands.spawn_probe_camera(transform, resolution, &mut images, &mut ssbo_assets)
    .insert(SomeComponent);
```

## Comparison

| Factor | Function | Trait Extension |
|--------|----------|-----------------|
| **API Feel** | Functional | OOP/Method-like |
| **Method Chaining** | ❌ No | ✅ Yes |
| **Complexity** | Low | Medium (trait + impl) |
| **Multiple Resources** | ✅ Clean | ⚠️ Awkward signature |
| **Discoverability** | Auto-complete works | Auto-complete works |
| **Bevy Convention** | Common for helpers | Common for built-ins |

## When to Use Trait Extensions

✅ **Good for:**
- Simple spawns (no asset creation)
- When you want method chaining
- When the API is the primary user-facing API
- When parameters are simple (Transform, simple types)

❌ **Not ideal for:**
- Complex setups requiring multiple `ResMut` parameters
- When the function signature becomes unwieldy
- Internal helper functions
- When the primary API is events/messages

## Recommendation for Probe Camera

**Keep the function approach** because:

1. **Complex signature** - Requires `&mut Assets<Image>` and `&mut Assets<ShaderStorageBuffer>`
   - Trait extension would have: `commands.spawn_probe_camera(transform, resolution, &mut images, &mut ssbo_assets)`
   - Less ergonomic than it looks

2. **Primary API is events** - Users spawn via `ProbeCameraCommand::Add`, not direct function calls
   - Function is primarily for internal use
   - Events are cleaner user-facing API

3. **System parameters** - The function is called from systems that already have these parameters
   - Function approach fits naturally
   - Trait extension doesn't add value when you're already in a system

4. **Bevy convention** - Complex entity creation with assets typically uses functions, not trait extensions
   - Built-in trait extensions (`Commands::spawn`) are for simple cases
   - Complex setup = helper function

## When Trait Extensions Make Sense

Example where trait extension IS worth it:

```rust
// Simple case - no assets, clean API
pub trait CameraCommands {
    fn spawn_orbit_camera(&mut self, target: Vec3, distance: f32) -> EntityCommands;
}

// Usage:
commands.spawn_orbit_camera(Vec3::ZERO, 10.0)
    .insert(MyComponent);
```

This works because:
- Simple parameters
- No asset creation
- Direct user-facing API
- Benefits from method chaining

## Hybrid Approach (If You Really Want It)

You can provide BOTH:

```rust
// Internal: Function (used by systems)
pub fn spawn_probe_camera(...) -> (Entity, Vec2) { ... }

// External: Trait extension (convenience for advanced users)
pub trait ProbeCameraCommands {
    fn spawn_probe_camera(...) -> EntityCommands {
        let (entity, _) = spawn_probe_camera(self, ...);
        // return EntityCommands for that entity
    }
}
```

But this adds complexity for marginal benefit since users primarily use events.
