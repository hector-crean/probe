use bevy::prelude::*;
use bevy::render::render_resource::*;
use std::collections::HashMap;

// ✅ ECS-idiomatic: Components, not trait objects
#[derive(Component)]
pub struct NodeId(pub u32);

#[derive(Component)]
pub struct NodeConnections {
    pub inputs: Vec<Entity>,
    pub outputs: Vec<Entity>,
}

#[derive(Component)]
pub struct NodePosition(pub Vec2);

#[derive(Component)]
pub struct NodeEnabled(pub bool);

// ✅ Specific node types as components
#[derive(Component)]
pub struct CameraSourceNode {
    pub camera_entity: Entity,
    pub render_target: Handle<Image>,
    pub resolution: UVec2,
    pub render_layers: RenderLayers,
}

#[derive(Component)]
pub struct PixelProbeNode {
    pub kernel_size: Vec2,
    pub sample_positions: Vec<Vec2>,
    pub interpolation: InterpolationType,
}

#[derive(Component)]
pub struct ConvolutionNode {
    pub kernel: Vec<f32>,
    pub kernel_size: UVec2,
    pub border_mode: BorderMode,
}

#[derive(Component)]
pub struct ThresholdNode {
    pub threshold: f32,
    pub comparison: ComparisonType,
}

// ✅ Data flow through components
#[derive(Component)]
pub struct NodeOutput<T> {
    pub data: T,
    pub frame: u64,
    pub dirty: bool,
}

#[derive(Component)]
pub struct ImageOutput {
    pub image: Handle<Image>,
    pub frame: u64,
    pub dirty: bool,
}

#[derive(Component)]
pub struct BufferOutput {
    pub buffer: Handle<Buffer>,
    pub frame: u64,
    pub dirty: bool,
}

#[derive(Component)]
pub struct PixelDataOutput {
    pub pixels: Vec<Vec4>,
    pub frame: u64,
    pub dirty: bool,
}

// ✅ Connection system using entities
#[derive(Component)]
pub struct NodeConnection {
    pub from_entity: Entity,
    pub to_entity: Entity,
    pub from_output: OutputSlot,
    pub to_input: InputSlot,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutputSlot {
    Image,
    Buffer,
    PixelData,
    Scalar,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InputSlot {
    Image,
    Buffer,
    PixelData,
    Scalar,
}

// ✅ Enums for node-specific data
#[derive(Debug, Clone)]
pub enum InterpolationType {
    Nearest,
    Linear,
    Cubic,
}

#[derive(Debug, Clone)]
pub enum BorderMode {
    Clamp,
    Repeat,
    Mirror,
}

#[derive(Debug, Clone)]
pub enum ComparisonType {
    Greater,
    Less,
    Equal,
    GreaterEqual,
    LessEqual,
}

// ✅ Events for node communication
#[derive(Event)]
pub struct NodeProcessed {
    pub entity: Entity,
    pub frame: u64,
}

#[derive(Event)]
pub struct NodeError {
    pub entity: Entity,
    pub error: String,
}

// ✅ Resources for global state
#[derive(Resource, Default)]
pub struct PipelineFrame {
    pub current: u64,
}

#[derive(Resource, Default)]
pub struct PipelineStats {
    pub nodes_processed: u32,
    pub processing_time: f32,
}

// ✅ Marker components for scheduling
#[derive(Component)]
pub struct SourceNode;

#[derive(Component)]
pub struct TransformNode;

#[derive(Component)]
pub struct ProbeNode;

#[derive(Component)]
pub struct SinkNode;

// ✅ Bundle for easy node creation
#[derive(Bundle)]
pub struct NodeBundle {
    pub id: NodeId,
    pub connections: NodeConnections,
    pub position: NodePosition,
    pub enabled: NodeEnabled,
}

impl Default for NodeBundle {
    fn default() -> Self {
        Self {
            id: NodeId(0),
            connections: NodeConnections {
                inputs: Vec::new(),
                outputs: Vec::new(),
            },
            position: NodePosition(Vec2::ZERO),
            enabled: NodeEnabled(true),
        }
    }
}

// ✅ Systems for node processing (instead of trait methods)
pub fn process_camera_source_nodes(
    mut source_nodes: Query<(Entity, &CameraSourceNode, &mut ImageOutput), With<SourceNode>>,
    mut frame: ResMut<PipelineFrame>,
    // ... other resources
) {
    for (entity, camera_node, mut output) in source_nodes.iter_mut() {
        // Process camera source node
        if output.frame < frame.current {
            // Render camera to texture
            // output.image = rendered_image;
            output.frame = frame.current;
            output.dirty = true;
        }
    }
}

pub fn process_pixel_probe_nodes(
    mut probe_nodes: Query<(Entity, &PixelProbeNode, &mut PixelDataOutput), With<ProbeNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, probe_node, mut output) in probe_nodes.iter_mut() {
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
                // Process the probe
                // output.pixels = extract_pixels(image_output.image, probe_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

pub fn process_convolution_nodes(
    mut conv_nodes: Query<(Entity, &ConvolutionNode, &mut ImageOutput), With<TransformNode>>,
    image_outputs: Query<&ImageOutput>,
    connections: Query<&NodeConnection>,
    frame: Res<PipelineFrame>,
) {
    for (entity, conv_node, mut output) in conv_nodes.iter_mut() {
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
                // Process convolution
                // output.image = apply_convolution(image_output.image, conv_node);
                output.frame = frame.current;
                output.dirty = true;
            }
        }
    }
}

// ✅ Helper functions for node creation
pub fn spawn_camera_source_node(
    commands: &mut Commands,
    camera_entity: Entity,
    resolution: UVec2,
) -> Entity {
    commands
        .spawn((
            NodeBundle::default(),
            CameraSourceNode {
                camera_entity,
                render_target: Handle::default(),
                resolution,
                render_layers: RenderLayers::layer(1),
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            SourceNode,
        ))
        .id()
}

pub fn spawn_pixel_probe_node(
    commands: &mut Commands,
    sample_positions: Vec<Vec2>,
    kernel_size: Vec2,
) -> Entity {
    commands
        .spawn((
            NodeBundle::default(),
            PixelProbeNode {
                kernel_size,
                sample_positions,
                interpolation: InterpolationType::Linear,
            },
            PixelDataOutput {
                pixels: Vec::new(),
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id()
}

pub fn connect_nodes(
    commands: &mut Commands,
    from_entity: Entity,
    to_entity: Entity,
    from_output: OutputSlot,
    to_input: InputSlot,
) -> Entity {
    commands
        .spawn(NodeConnection {
            from_entity,
            to_entity,
            from_output,
            to_input,
        })
        .id()
}

// ✅ System sets for proper ordering
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum NodeProcessingSet {
    Sources,
    Transforms,
    Probes,
    Sinks,
    Cleanup,
}
