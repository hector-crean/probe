use bevy::prelude::*;
use bevy::render::RenderApp;

use crate::pipeline::ecs_nodes::*;

pub struct PipelinePlugin;

impl Plugin for PipelinePlugin {
    fn build(&self, app: &mut App) {
        app
            // Resources
            .init_resource::<PipelineFrame>()
            .init_resource::<PipelineStats>()
            // Events
            .add_event::<NodeProcessed>()
            .add_event::<NodeError>()
            // System sets for proper ordering
            .configure_sets(
                Update,
                (
                    NodeProcessingSet::Sources,
                    NodeProcessingSet::Transforms,
                    NodeProcessingSet::Probes,
                    NodeProcessingSet::Sinks,
                    NodeProcessingSet::Cleanup,
                )
                    .chain(),
            )
            // Source node systems
            .add_systems(
                Update,
                (
                    process_camera_source_nodes,
                    process_procedural_source_nodes,
                    process_scene_query_source_nodes,
                )
                    .in_set(NodeProcessingSet::Sources),
            )
            // Transform node systems
            .add_systems(
                Update,
                (
                    process_convolution_nodes,
                    process_threshold_nodes,
                    process_color_space_nodes,
                    process_math_nodes,
                    process_temporal_filter_nodes,
                )
                    .in_set(NodeProcessingSet::Transforms),
            )
            // Probe node systems
            .add_systems(
                Update,
                (
                    process_pixel_probe_nodes,
                    process_depth_probe_nodes,
                    process_geometry_probe_nodes,
                    process_statistics_probe_nodes,
                    process_motion_probe_nodes,
                )
                    .in_set(NodeProcessingSet::Probes),
            )
            // Sink node systems
            .add_systems(
                Update,
                (
                    process_file_output_nodes,
                    process_display_nodes,
                    process_network_output_nodes,
                )
                    .in_set(NodeProcessingSet::Sinks),
            )
            // Cleanup and maintenance
            .add_systems(
                Update,
                (
                    update_pipeline_frame,
                    cleanup_old_outputs,
                    update_pipeline_stats,
                )
                    .in_set(NodeProcessingSet::Cleanup),
            )
            // Visual editor systems (if enabled)
            .add_systems(
                Update,
                (
                    handle_node_editor_ui,
                    handle_node_connections,
                    handle_node_selection,
                )
                    .run_if(resource_exists::<NodeEditorState>),
            );

        // Sub-app for render world processing
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                bevy::render::Render,
                (extract_node_data, process_render_nodes, readback_probe_data).chain(),
            );
        }
    }
}

// ✅ ECS systems replace the centralized executor
fn update_pipeline_frame(mut frame: ResMut<PipelineFrame>, time: Res<Time>) {
    frame.current += 1;
}

fn cleanup_old_outputs(
    mut image_outputs: Query<&mut ImageOutput>,
    mut buffer_outputs: Query<&mut BufferOutput>,
    mut pixel_outputs: Query<&mut PixelDataOutput>,
    frame: Res<PipelineFrame>,
) {
    // Clean up outputs that are more than N frames old
    let cleanup_threshold = frame.current.saturating_sub(5);

    for mut output in image_outputs.iter_mut() {
        if output.frame < cleanup_threshold {
            output.dirty = false;
        }
    }

    for mut output in buffer_outputs.iter_mut() {
        if output.frame < cleanup_threshold {
            output.dirty = false;
        }
    }

    for mut output in pixel_outputs.iter_mut() {
        if output.frame < cleanup_threshold {
            output.dirty = false;
        }
    }
}

fn update_pipeline_stats(
    mut stats: ResMut<PipelineStats>,
    mut events: EventReader<NodeProcessed>,
    time: Res<Time>,
) {
    stats.nodes_processed = events.read().count() as u32;
    stats.processing_time = time.delta_secs();
}

// ✅ Render world systems for GPU operations
fn extract_node_data(// Extract relevant node data to render world
) {
    // This would extract node data needed for GPU processing
}

fn process_render_nodes(// Process nodes that need GPU compute
) {
    // Handle GPU compute shaders, render targets, etc.
}

fn readback_probe_data(// Handle GPU readback for probe nodes
) {
    // Async GPU readback for probe data
}

// ✅ Helper systems for specific node types
fn process_procedural_source_nodes(
    mut source_nodes: Query<(Entity, &ProceduralSourceNode, &mut ImageOutput), With<SourceNode>>,
    frame: Res<PipelineFrame>,
) {
    for (entity, procedural_node, mut output) in source_nodes.iter_mut() {
        if output.frame < frame.current {
            // Generate procedural texture
            // output.image = generate_procedural_texture(procedural_node);
            output.frame = frame.current;
            output.dirty = true;
        }
    }
}

fn process_scene_query_source_nodes(
    mut source_nodes: Query<(Entity, &SceneQuerySourceNode, &mut BufferOutput), With<SourceNode>>,
    frame: Res<PipelineFrame>,
) {
    for (entity, query_node, mut output) in source_nodes.iter_mut() {
        if output.frame < frame.current {
            // Query scene for data
            // output.buffer = query_scene_data(query_node);
            output.frame = frame.current;
            output.dirty = true;
        }
    }
}

fn process_threshold_nodes(
    mut threshold_nodes: Query<(Entity, &ThresholdNode, &mut ImageOutput), With<TransformNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, threshold_node, mut output) in threshold_nodes.iter_mut() {
        if output.frame >= frame.current {
            continue;
        }

        // Find connected image input
        let input_image = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Image)
            .and_then(|conn| image_outputs.get(conn.from_entity).ok());

        if let Some(image_output) = input_image {
            if image_output.dirty {
                // Apply threshold
                // output.image = apply_threshold(image_output.image, threshold_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

fn process_color_space_nodes(
    mut color_nodes: Query<(Entity, &ColorSpaceNode, &mut ImageOutput), With<TransformNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, color_node, mut output) in color_nodes.iter_mut() {
        if output.frame >= frame.current {
            continue;
        }

        let input_image = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Image)
            .and_then(|conn| image_outputs.get(conn.from_entity).ok());

        if let Some(image_output) = input_image {
            if image_output.dirty {
                // Convert color space
                // output.image = convert_color_space(image_output.image, color_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

fn process_math_nodes(
    mut math_nodes: Query<(Entity, &MathNode, &mut ScalarOutput), With<TransformNode>>,
    scalar_outputs: Query<&ScalarOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, math_node, mut output) in math_nodes.iter_mut() {
        if output.frame >= frame.current {
            continue;
        }

        // Math nodes can have multiple inputs
        let inputs: Vec<f32> = connections
            .iter()
            .filter(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Scalar)
            .filter_map(|conn| scalar_outputs.get(conn.from_entity).ok())
            .filter(|output| output.dirty)
            .map(|output| output.value)
            .collect();

        if !inputs.is_empty() {
            // Apply math operation
            // output.value = apply_math_operation(inputs, math_node);
            output.frame = frame.current;
            output.dirty = true;
        }
    }
}

fn process_temporal_filter_nodes(
    mut filter_nodes: Query<
        (Entity, &mut TemporalFilterNode, &mut ImageOutput),
        With<TransformNode>,
    >,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, mut filter_node, mut output) in filter_nodes.iter_mut() {
        if output.frame >= frame.current {
            continue;
        }

        let input_image = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Image)
            .and_then(|conn| image_outputs.get(conn.from_entity).ok());

        if let Some(image_output) = input_image {
            if image_output.dirty {
                // Apply temporal filtering
                // output.image = apply_temporal_filter(image_output.image, &mut filter_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

fn process_depth_probe_nodes(
    mut probe_nodes: Query<(Entity, &DepthProbeNode, &mut BufferOutput), With<ProbeNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, probe_node, mut output) in probe_nodes.iter_mut() {
        if output.frame >= frame.current {
            continue;
        }

        let input_image = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Image)
            .and_then(|conn| image_outputs.get(conn.from_entity).ok());

        if let Some(image_output) = input_image {
            if image_output.dirty {
                // Extract depth data
                // output.buffer = extract_depth_data(image_output.image, probe_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

fn process_geometry_probe_nodes(
    mut probe_nodes: Query<(Entity, &GeometryProbeNode, &mut BufferOutput), With<ProbeNode>>,
    frame: Res<PipelineFrame>,
) {
    for (entity, probe_node, mut output) in probe_nodes.iter_mut() {
        if output.frame < frame.current {
            // Ray casting for geometry probing
            // output.buffer = cast_rays(probe_node);
            output.frame = frame.current;
            output.dirty = true;
        }
    }
}

fn process_statistics_probe_nodes(
    mut probe_nodes: Query<(Entity, &StatisticsProbeNode, &mut BufferOutput), With<ProbeNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, probe_node, mut output) in probe_nodes.iter_mut() {
        if output.frame >= frame.current {
            continue;
        }

        let input_image = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Image)
            .and_then(|conn| image_outputs.get(conn.from_entity).ok());

        if let Some(image_output) = input_image {
            if image_output.dirty {
                // Compute statistics
                // output.buffer = compute_statistics(image_output.image, probe_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

fn process_motion_probe_nodes(
    mut probe_nodes: Query<(Entity, &mut MotionProbeNode, &mut BufferOutput), With<ProbeNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, mut probe_node, mut output) in probe_nodes.iter_mut() {
        if output.frame >= frame.current {
            continue;
        }

        let input_image = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Image)
            .and_then(|conn| image_outputs.get(conn.from_entity).ok());

        if let Some(image_output) = input_image {
            if image_output.dirty {
                // Track motion
                // output.buffer = track_motion(image_output.image, &mut probe_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

fn process_file_output_nodes(
    mut output_nodes: Query<(Entity, &FileOutputNode), With<SinkNode>>,
    image_outputs: Query<&ImageOutput>,
    buffer_outputs: Query<&BufferOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, output_node) in output_nodes.iter_mut() {
        // Find connected inputs and save to file
        for connection in connections.iter().filter(|conn| conn.to_entity == entity) {
            match connection.to_input {
                InputSlot::Image => {
                    if let Ok(image_output) = image_outputs.get(connection.from_entity) {
                        if image_output.dirty {
                            // Save image to file
                            // save_image_to_file(image_output.image, &output_node.path);
                        }
                    }
                }
                InputSlot::Buffer => {
                    if let Ok(buffer_output) = buffer_outputs.get(connection.from_entity) {
                        if buffer_output.dirty {
                            // Save buffer to file
                            // save_buffer_to_file(buffer_output.buffer, &output_node.path);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn process_display_nodes(
    mut display_nodes: Query<(Entity, &DisplayNode), With<SinkNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, display_node) in display_nodes.iter_mut() {
        let input_image = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Image)
            .and_then(|conn| image_outputs.get(conn.from_entity).ok());

        if let Some(image_output) = input_image {
            if image_output.dirty {
                // Display image on screen
                // display_image(image_output.image, display_node);
            }
        }
    }
}

fn process_network_output_nodes(
    mut network_nodes: Query<(Entity, &NetworkOutputNode), With<SinkNode>>,
    buffer_outputs: Query<&BufferOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, network_node) in network_nodes.iter_mut() {
        let input_buffer = connections
            .iter()
            .find(|conn| conn.to_entity == entity && conn.to_input == InputSlot::Buffer)
            .and_then(|conn| buffer_outputs.get(conn.from_entity).ok());

        if let Some(buffer_output) = input_buffer {
            if buffer_output.dirty {
                // Send data over network
                // send_data_over_network(buffer_output.buffer, network_node);
            }
        }
    }
}

// Visual editor systems
fn handle_node_editor_ui(// Handle visual node editor UI
) {
    // Implementation for visual node editor
}

fn handle_node_connections(// Handle creating/removing connections in UI
) {
    // Implementation for connection management
}

fn handle_node_selection(// Handle node selection in UI
) {
    // Implementation for node selection
}

// Additional node types that would need components
#[derive(Component)]
struct ProceduralSourceNode {
    generator_type: ProceduralType,
    parameters: std::collections::HashMap<String, f32>,
}

#[derive(Component)]
struct SceneQuerySourceNode {
    query_type: SceneQueryType,
    spatial_bounds: BoundingBox,
}

#[derive(Component)]
struct ColorSpaceNode {
    from_space: ColorSpace,
    to_space: ColorSpace,
}

#[derive(Component)]
struct MathNode {
    operation: MathOperation,
    operands: Vec<f32>,
}

#[derive(Component)]
struct TemporalFilterNode {
    filter_type: TemporalFilterType,
    window_size: usize,
    alpha: f32,
    history: Vec<Handle<Image>>,
}

#[derive(Component)]
struct DepthProbeNode {
    positions: Vec<Vec2>,
    depth_comparison: DepthTest,
}

#[derive(Component)]
struct GeometryProbeNode {
    ray_origins: Vec<Vec3>,
    ray_directions: Vec<Vec3>,
    max_distance: f32,
}

#[derive(Component)]
struct StatisticsProbeNode {
    region: Region,
    stats_type: StatisticsType,
}

#[derive(Component)]
struct MotionProbeNode {
    tracking_points: Vec<Vec2>,
    temporal_window: usize,
    motion_type: MotionType,
    previous_frames: Vec<Handle<Image>>,
}

#[derive(Component)]
struct FileOutputNode {
    path: String,
    format: FileFormat,
}

#[derive(Component)]
struct DisplayNode {
    window_id: WindowId,
    position: Vec2,
    size: Vec2,
}

#[derive(Component)]
struct NetworkOutputNode {
    endpoint: String,
    protocol: NetworkProtocol,
}

#[derive(Component)]
struct ScalarOutput {
    value: f32,
    frame: u64,
    dirty: bool,
}

// Additional output types would need similar components...

// Placeholder types
#[derive(Component)]
struct NodeEditorState;

type ProceduralType = u32;
type SceneQueryType = u32;
type BoundingBox = ();
type ColorSpace = u32;
type MathOperation = u32;
type TemporalFilterType = u32;
type DepthTest = u32;
type Region = ();
type StatisticsType = u32;
type MotionType = u32;
type FileFormat = u32;
type WindowId = u32;
type NetworkProtocol = u32;
