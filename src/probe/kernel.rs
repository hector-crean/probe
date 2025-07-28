use bevy::prelude::*;

pub struct KernelPlugin;

impl KernelPlugin {
    /// Sets up the kernel visualization UI
    fn setup_kernel_visualization_ui(mut commands: Commands) {
        // Create a UI panel to show kernel data as colored grid
        commands
            .spawn((
                Node {
                    position_type: PositionType::Absolute,
                    top: Val::Px(10.0),
                    right: Val::Px(10.0),
                    width: Val::Px(200.0),
                    padding: UiRect::all(Val::Px(10.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                KernelVisualizationUI,
            ))
            .with_children(|parent| {
                // Title
                parent.spawn(Text::new("Kernel Data"));

                // Grid container that will hold the colored squares
                parent.spawn((
                    Node {
                        display: Display::Grid,
                        grid_template_columns: vec![GridTrack::px(30.0); 3], // 3x3 grid initially
                        grid_template_rows: vec![GridTrack::px(30.0); 3],
                        column_gap: Val::Px(2.0),
                        row_gap: Val::Px(2.0),
                        margin: UiRect::top(Val::Px(10.0)),
                        ..default()
                    },
                    KernelGrid,
                ));
            });
    }

    /// Updates the kernel visualization UI with current data
    fn update_kernel_visualization_ui(
        mut commands: Commands,
        grid_query: Query<Entity, With<KernelGrid>>,
        kernel_data: Res<KernelDataResource>,
        children_query: Query<&Children>,
        mut background_query: Query<&mut BackgroundColor>,
        mut text_query: Query<&mut Text>,
    ) {
        // Only update if kernel data is not empty
        if kernel_data.data.is_empty() {
            return;
        }

        for grid_entity in grid_query.iter() {
            // Check if grid already has children (squares)
            if let Ok(children) = children_query.get(grid_entity) {
                if children.len() != kernel_data.data.len() {
                    // Grid size changed, need to rebuild
                    for child in children.iter() {
                        commands.entity(child).despawn();
                    }
                    Self::create_kernel_grid(&mut commands, grid_entity, &kernel_data);
                } else {
                    // Update existing squares
                    for (index, child) in children.iter().enumerate() {
                        if index < kernel_data.data.len() {
                            let pixel = &kernel_data.data[index];
                            let color = Color::srgba(
                                pixel.x.clamp(0.0, 1.0),
                                pixel.y.clamp(0.0, 1.0),
                                pixel.z.clamp(0.0, 1.0),
                                1.0,
                            );

                            if let Ok(mut bg_color) = background_query.get_mut(child) {
                                *bg_color = BackgroundColor(color);
                            }

                            // Update text in grandchildren if exists
                            if let Ok(child_children) = children_query.get(child) {
                                for grandchild in child_children.iter() {
                                    if let Ok(mut text) = text_query.get_mut(grandchild) {
                                        text.0 = format!("{:.1}", pixel.x);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // No children yet, create initial grid
                Self::create_kernel_grid(&mut commands, grid_entity, &kernel_data);
            }
        }
    }

    /// Helper function to create the kernel grid
    fn create_kernel_grid(
        commands: &mut Commands,
        grid_entity: Entity,
        kernel_data: &KernelDataResource,
    ) {
        commands.entity(grid_entity).with_children(|parent| {
            for (_index, pixel) in kernel_data.data.iter().enumerate() {
                let color = Color::srgba(
                    pixel.x.clamp(0.0, 1.0),
                    pixel.y.clamp(0.0, 1.0),
                    pixel.z.clamp(0.0, 1.0),
                    1.0,
                );

                parent
                    .spawn((
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            border: UiRect::all(Val::Px(1.0)),
                            ..default()
                        },
                        BackgroundColor(color),
                        BorderColor(Color::WHITE),
                    ))
                    .with_children(|cell| {
                        cell.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                bottom: Val::Px(1.0),
                                right: Val::Px(1.0),
                                ..default()
                            },
                            Text::new(format!("{:.1}", pixel.x)),
                        ));
                    });
            }
        });
    }
}

impl Plugin for KernelPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<KernelDataResource>()
            .add_systems(Startup, Self::setup_kernel_visualization_ui)
            .add_systems(Update, (Self::update_kernel_visualization_ui,));
    }
}

/// Resource to store the current kernel data for UI visualization
#[derive(Resource, Default)]
pub struct KernelDataResource {
    pub data: Vec<Vec4>,
    pub kernel_size: Vec2,
}

/// Marker component for the kernel visualization UI
#[derive(Component)]
pub struct KernelVisualizationUI;

/// Marker component for the kernel grid container
#[derive(Component)]
pub struct KernelGrid;
