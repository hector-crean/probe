//! Burn ML integration example.
//!
//! This example demonstrates:
//! - Processing probe kernel data through a Burn neural network
//! - Creating a simple color classification model
//! - Converting between Bevy and Burn tensor formats
//! - Displaying ML results in a UI panel
//!
//! Run with: `cargo run --example burn_integration`
//! (burn feature is enabled by default)

use bevy::{camera::visibility::RenderLayers, prelude::*};
use probe::prelude::*;

fn main() {
    #[cfg(not(feature = "burn"))]
    {
        eprintln!("This example requires the `burn` feature.");
        eprintln!("Run with: cargo run --example burn_integration --features burn");
        return;
    }

    #[cfg(feature = "burn")]
    {
        use probe::camera::CameraSettingsImpl;

        App::new()
            .init_resource::<CameraSettingsImpl>()
            .init_resource::<ModelOutput>()
            .add_plugins((
                DefaultPlugins,
                OrbitCameraControllerPlugin::<CameraSettingsImpl>::default(),
                ProbeCameraControllerPlugin,
                ProbeToolPlugin,
            ))
            .add_systems(Startup, (setup_scene, setup_burn_model, setup_ui))
            .add_systems(Update, (process_kernel_with_burn, update_ui))
            .run();
    }
}

// ============================================================================
// Burn Model Definition
// ============================================================================

#[cfg(feature = "burn")]
mod burn_model {
    use burn::{
        backend::Wgpu,
        nn::{Linear, LinearConfig, Relu},
        prelude::*,
        tensor::Tensor,
    };

    type Backend = Wgpu;

    /// A simple neural network that analyzes kernel colors.
    ///
    /// Architecture:
    /// - Input: Flattened kernel (4 channels × H × W)
    /// - Hidden: 32 units with ReLU
    /// - Output: 4 values (average R, G, B, dominant hue class)
    #[derive(Module, Debug)]
    pub struct ColorAnalyzer<B: burn::tensor::backend::Backend> {
        linear1: Linear<B>,
        activation: Relu,
        linear2: Linear<B>,
    }

    impl ColorAnalyzer<Backend> {
        /// Creates a new color analyzer for the given kernel size.
        pub fn new(
            device: &<Backend as burn::tensor::backend::Backend>::Device,
            kernel_width: usize,
            kernel_height: usize,
        ) -> Self {
            let input_size = 4 * kernel_width * kernel_height; // RGBA channels
            let hidden_size = 32;
            let output_size = 4; // [avg_r, avg_g, avg_b, dominant_class]

            Self {
                linear1: LinearConfig::new(input_size, hidden_size).init(device),
                activation: Relu::new(),
                linear2: LinearConfig::new(hidden_size, output_size).init(device),
            }
        }

        /// Forward pass through the network.
        ///
        /// Input shape: [batch, channels, height, width]
        /// Output shape: [batch, 4]
        pub fn forward(&self, input: Tensor<Backend, 4>) -> Tensor<Backend, 2> {
            let [batch, channels, height, width] = input.dims();
            let flattened = input.reshape([batch, channels * height * width]);

            let x = self.linear1.forward(flattened);
            let x = self.activation.forward(x);
            self.linear2.forward(x)
        }
    }

    /// Wrapper for Burn model and device.
    pub struct BurnModelWrapper {
        pub model: ColorAnalyzer<Backend>,
        pub device: <Backend as burn::tensor::backend::Backend>::Device,
    }
}

#[cfg(feature = "burn")]
use burn_model::BurnModelWrapper;

// ============================================================================
// Model Output Resource (for UI)
// ============================================================================

/// Stores the latest model output for UI display.
#[derive(Resource, Default)]
struct ModelOutput {
    /// RGB output from the model
    rgb: [f32; 3],
    /// Classification value
    class_value: f32,
    /// Raw kernel pixels (for visualization)
    kernel_pixels: Vec<Vec4>,
    /// Kernel dimensions
    kernel_size: (usize, usize),
    /// Whether we have valid data
    has_data: bool,
}

// ============================================================================
// UI Components
// ============================================================================

#[derive(Component)]
struct OutputPanel;

#[derive(Component)]
struct RgbValueText;

#[derive(Component)]
struct ClassValueText;

#[derive(Component)]
struct KernelGridContainer;

#[derive(Component)]
#[allow(dead_code)]
struct KernelPixelCell(usize);

// ============================================================================
// Systems
// ============================================================================

#[cfg(feature = "burn")]
fn setup_burn_model(world: &mut World) {
    use burn::backend::wgpu::WgpuDevice;

    let device = WgpuDevice::default();
    let model = burn_model::ColorAnalyzer::new(&device, 3, 3);

    info!("🔥 Burn model initialized on {:?}", device);

    world.insert_non_send_resource(BurnModelWrapper { model, device });
}

#[cfg(feature = "burn")]
fn process_kernel_with_burn(
    mut reader: MessageReader<ProbeCameraOutputMessage>,
    burn_model: Option<NonSend<BurnModelWrapper>>,
    mut output: ResMut<ModelOutput>,
) {
    use burn::backend::Wgpu;

    let Some(burn_model) = burn_model else {
        return;
    };

    for message in reader.read() {
        if let Some(kernel_data) = message.to_kernel_data() {
            // Store kernel for visualization
            let ProbeCameraOutputMessage::KernelChanged {
                data, kernel_size, ..
            } = message;
            output.kernel_pixels = data.clone();
            output.kernel_size = (kernel_size.x as usize, kernel_size.y as usize);

            // Convert to Burn tensor with batch dimension
            let tensor = kernel_data.to_burn_tensor_batched::<Wgpu>(&burn_model.device);

            // Run through the model
            let model_output = burn_model.model.forward(tensor);

            // Get the results back to CPU
            let output_data = model_output.to_data();
            let values: Vec<f32> = output_data.to_vec().unwrap();

            // Update the resource for UI
            output.rgb = [values[0], values[1], values[2]];
            output.class_value = values[3];
            output.has_data = true;

            info!(
                "🎨 Model: R={:.3}, G={:.3}, B={:.3}, class={:.3}",
                values[0], values[1], values[2], values[3]
            );
        }
    }
}

fn setup_ui(mut commands: Commands) {
    // Main UI panel
    commands
        .spawn((
            OutputPanel,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(20.0),
                top: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(12.0),
                min_width: Val::Px(280.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.1, 0.15, 0.95)),
            BorderRadius::all(Val::Px(8.0)),
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("🔥 Burn ML Output"),
                TextFont {
                    font_size: 20.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.8, 0.3)),
            ));

            // Divider
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(1.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.2)),
            ));

            // Kernel visualization section
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },))
                .with_children(|section| {
                    section.spawn((
                        Text::new("Kernel Input"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                    ));

                    // Kernel grid container
                    section.spawn((
                        KernelGridContainer,
                        Node {
                            display: Display::Grid,
                            grid_template_columns: RepeatedGridTrack::flex(3, 1.0),
                            column_gap: Val::Px(4.0),
                            row_gap: Val::Px(4.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.3)),
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                });

            // Model output section
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },))
                .with_children(|section| {
                    section.spawn((
                        Text::new("Model Output"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                    ));

                    // RGB values
                    section.spawn((
                        RgbValueText,
                        Text::new("R: --  G: --  B: --"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));

                    // Class value
                    section.spawn((
                        ClassValueText,
                        Text::new("Class: --"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
                });

            // Color preview
            parent
                .spawn((Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(8.0),
                    ..default()
                },))
                .with_children(|section| {
                    section.spawn((
                        Text::new("Predicted Color"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.7)),
                    ));

                    // Color swatch (will be updated)
                    section.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            height: Val::Px(40.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                        BorderRadius::all(Val::Px(4.0)),
                    ));
                });

            // Instructions
            parent.spawn((
                Text::new("Move probe camera to analyze colors"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
            ));
        });
}

fn update_ui(
    output: Res<ModelOutput>,
    mut rgb_text: Single<&mut Text, (With<RgbValueText>, Without<ClassValueText>)>,
    mut class_text: Single<&mut Text, (With<ClassValueText>, Without<RgbValueText>)>,
    kernel_container: Single<(Entity, &mut Node), With<KernelGridContainer>>,
    kernel_cells: Query<Entity, With<KernelPixelCell>>,
    mut commands: Commands,
    mut color_swatch: Query<
        &mut BackgroundColor,
        (Without<KernelGridContainer>, Without<OutputPanel>),
    >,
) {
    if !output.has_data {
        return;
    }

    // Update RGB text
    **rgb_text = Text::new(format!(
        "R: {:.2}  G: {:.2}  B: {:.2}",
        output.rgb[0], output.rgb[1], output.rgb[2]
    ));

    // Update class text
    **class_text = Text::new(format!("Class: {:.3}", output.class_value));

    // Update kernel grid
    {
        let (container_entity, mut node) = kernel_container.into_inner();
        let (width, _height) = output.kernel_size;

        // Update grid columns if size changed
        node.grid_template_columns = RepeatedGridTrack::flex(width as u16, 1.0);

        // Remove old cells
        for entity in kernel_cells.iter() {
            commands.entity(entity).despawn();
        }

        // Add new cells
        commands.entity(container_entity).with_children(|parent| {
            for (i, pixel) in output.kernel_pixels.iter().enumerate() {
                let color = Color::srgb(
                    pixel.x.clamp(0.0, 1.0),
                    pixel.y.clamp(0.0, 1.0),
                    pixel.z.clamp(0.0, 1.0),
                );

                parent.spawn((
                    KernelPixelCell(i),
                    Node {
                        width: Val::Px(24.0),
                        height: Val::Px(24.0),
                        ..default()
                    },
                    BackgroundColor(color),
                    BorderRadius::all(Val::Px(2.0)),
                ));
            }
        });
    }

    // Update color swatch (find the one that's not the kernel container)
    // The color swatch is the last background with specific size
    for mut bg in color_swatch.iter_mut() {
        // Only update if it looks like our swatch (approximation)
        if bg.0 == Color::srgb(0.3, 0.3, 0.3) || output.has_data {
            // Clamp and convert model output to color
            let r = output.rgb[0].clamp(0.0, 1.0);
            let g = output.rgb[1].clamp(0.0, 1.0);
            let b = output.rgb[2].clamp(0.0, 1.0);
            *bg = BackgroundColor(Color::srgb(r, g, b));
        }
    }
}

// ============================================================================
// Scene Setup
// ============================================================================

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event_writer: MessageWriter<ProbeCameraCommand>,
) {
    // Main camera - NOTE: MainCamera marker is required for probe interaction
    commands.spawn((
        OrbitCameraController::default(),
        Camera {
            order: 0,
            ..default()
        },
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0.0, 0.0, 25.0)).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera, // Required for FrustumNearPlaneInteractionPlugin
        RenderLayers::layer(0),
    ));

    // Lighting
    let light_layers = RenderLayers::layer(0).with(1);
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.9, 0.9, 1.0),
        brightness: 0.4,
        affects_lightmapped_meshes: true,
    });
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.9),
            illuminance: 12000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
        light_layers,
    ));

    // Colored cubes
    for y in -3..3 {
        for z in -3..3 {
            let hue = ((y + 3) * 6 + (z + 3)) as f32 * 10.0;
            let color = Color::hsl(hue % 360.0, 0.8, 0.5);

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(0.8, 0.8, 0.8))),
                MeshMaterial3d(materials.add(color)),
                Transform::from_xyz(0.0, y as f32, z as f32),
                RenderLayers::layer(0).with(1),
            ));
        }
    }

    // Spawn probe camera
    event_writer.write(ProbeCameraCommand::Add {
        transform: Transform::from_xyz(4.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        resolution: UVec2::new(512, 512),
    });

    info!("🎮 Scene setup complete. Move the probe camera to analyze different colors.");
}
