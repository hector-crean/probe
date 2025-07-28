use crate::pipeline::ecs_nodes::*;
use crate::pipeline::plugin::*;
use bevy::prelude::*;

// ✅ ECS-style pipeline setup - much cleaner than centralized executor
pub fn setup_computer_vision_pipeline(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Create a camera source node
    let camera_source = commands
        .spawn((
            NodeBundle::default(),
            CameraSourceNode {
                camera_entity: Entity::from_raw(0), // Would be actual camera entity
                render_target: images.add(Image::default()),
                resolution: UVec2::new(1920, 1080),
                render_layers: RenderLayers::layer(1),
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            SourceNode,
        ))
        .id();

    // Create edge detection (convolution) node
    let edge_detection = commands
        .spawn((
            NodeBundle::default(),
            ConvolutionNode {
                // Sobel edge detection kernel
                kernel: vec![-1.0, -2.0, -1.0, 0.0, 0.0, 0.0, 1.0, 2.0, 1.0],
                kernel_size: UVec2::new(3, 3),
                border_mode: BorderMode::Clamp,
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    // Create feature detection probe
    let feature_probe = commands
        .spawn((
            NodeBundle::default(),
            PixelProbeNode {
                kernel_size: Vec2::new(5.0, 5.0),
                sample_positions: vec![
                    Vec2::new(0.25, 0.25), // Top-left quadrant
                    Vec2::new(0.75, 0.25), // Top-right quadrant
                    Vec2::new(0.25, 0.75), // Bottom-left quadrant
                    Vec2::new(0.75, 0.75), // Bottom-right quadrant
                    Vec2::new(0.5, 0.5),   // Center
                ],
                interpolation: InterpolationType::Linear,
            },
            PixelDataOutput {
                pixels: Vec::new(),
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id();

    // Connect the nodes - this is where ECS shines!
    connect_nodes(
        &mut commands,
        camera_source,
        edge_detection,
        OutputSlot::Image,
        InputSlot::Image,
    );

    connect_nodes(
        &mut commands,
        edge_detection,
        feature_probe,
        OutputSlot::Image,
        InputSlot::Image,
    );
}

// ✅ Motion tracking pipeline
pub fn setup_motion_tracking_pipeline(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    // Camera source
    let camera_source =
        spawn_camera_source_node(&mut commands, Entity::from_raw(0), UVec2::new(1280, 720));

    // Motion probe with temporal tracking
    let motion_probe = commands
        .spawn((
            NodeBundle::default(),
            MotionProbeNode {
                tracking_points: vec![
                    Vec2::new(0.3, 0.3),
                    Vec2::new(0.7, 0.3),
                    Vec2::new(0.5, 0.7),
                ],
                temporal_window: 5,
                motion_type: MotionType::OpticalFlow,
                previous_frames: Vec::new(),
            },
            BufferOutput {
                buffer: Handle::default(),
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id();

    // Velocity calculation (math node)
    let velocity_calc = commands
        .spawn((
            NodeBundle::default(),
            MathNode {
                operation: MathOperation::Derivative,
                operands: vec![],
            },
            ScalarOutput {
                value: 0.0,
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    // Display node to show results
    let display_node = commands
        .spawn((
            NodeBundle::default(),
            DisplayNode {
                window_id: WindowId::primary(),
                position: Vec2::new(100.0, 100.0),
                size: Vec2::new(400.0, 300.0),
            },
            SinkNode,
        ))
        .id();

    // Connect the pipeline
    connect_nodes(
        &mut commands,
        camera_source,
        motion_probe,
        OutputSlot::Image,
        InputSlot::Image,
    );

    connect_nodes(
        &mut commands,
        motion_probe,
        velocity_calc,
        OutputSlot::Buffer,
        InputSlot::Scalar,
    );

    connect_nodes(
        &mut commands,
        camera_source,
        display_node,
        OutputSlot::Image,
        InputSlot::Image,
    );
}

// ✅ Multi-camera depth analysis pipeline
pub fn setup_depth_analysis_pipeline(mut commands: Commands, camera_entities: Vec<Entity>) {
    let mut camera_sources = Vec::new();

    // Create multiple camera sources
    for (i, camera_entity) in camera_entities.iter().enumerate() {
        let camera_source = commands
            .spawn((
                NodeBundle::default(),
                CameraSourceNode {
                    camera_entity: *camera_entity,
                    render_target: Handle::default(),
                    resolution: UVec2::new(1024, 1024),
                    render_layers: RenderLayers::layer(i as u8 + 1),
                },
                ImageOutput {
                    image: Handle::default(),
                    frame: 0,
                    dirty: false,
                },
                SourceNode,
            ))
            .id();

        camera_sources.push(camera_source);
    }

    // Depth probe for each camera
    let mut depth_probes = Vec::new();
    for camera_source in &camera_sources {
        let depth_probe = commands
            .spawn((
                NodeBundle::default(),
                DepthProbeNode {
                    positions: vec![
                        Vec2::new(0.5, 0.5), // Center point
                        Vec2::new(0.3, 0.3), // Sample points
                        Vec2::new(0.7, 0.7),
                    ],
                    depth_comparison: DepthTest::Less,
                },
                BufferOutput {
                    buffer: Handle::default(),
                    frame: 0,
                    dirty: false,
                },
                ProbeNode,
            ))
            .id();

        // Connect camera to depth probe
        connect_nodes(
            &mut commands,
            *camera_source,
            depth_probe,
            OutputSlot::Image,
            InputSlot::Image,
        );

        depth_probes.push(depth_probe);
    }

    // Statistics node to analyze depth data
    let depth_stats = commands
        .spawn((
            NodeBundle::default(),
            StatisticsProbeNode {
                region: Region::FullImage,
                stats_type: StatisticsType::DepthHistogram,
            },
            BufferOutput {
                buffer: Handle::default(),
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id();

    // Connect all depth probes to statistics
    for depth_probe in depth_probes {
        connect_nodes(
            &mut commands,
            depth_probe,
            depth_stats,
            OutputSlot::Buffer,
            InputSlot::Buffer,
        );
    }

    // File output for analysis results
    let file_output = commands
        .spawn((
            NodeBundle::default(),
            FileOutputNode {
                path: "depth_analysis.json".to_string(),
                format: FileFormat::Json,
            },
            SinkNode,
        ))
        .id();

    connect_nodes(
        &mut commands,
        depth_stats,
        file_output,
        OutputSlot::Buffer,
        InputSlot::Buffer,
    );
}

// ✅ Real-time image processing pipeline
pub fn setup_realtime_processing_pipeline(mut commands: Commands, camera_entity: Entity) {
    // Camera source
    let camera_source =
        spawn_camera_source_node(&mut commands, camera_entity, UVec2::new(1920, 1080));

    // Gaussian blur
    let blur_node = commands
        .spawn((
            NodeBundle::default(),
            ConvolutionNode {
                kernel: vec![
                    1.0 / 16.0,
                    2.0 / 16.0,
                    1.0 / 16.0,
                    2.0 / 16.0,
                    4.0 / 16.0,
                    2.0 / 16.0,
                    1.0 / 16.0,
                    2.0 / 16.0,
                    1.0 / 16.0,
                ],
                kernel_size: UVec2::new(3, 3),
                border_mode: BorderMode::Clamp,
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    // Color space conversion
    let color_convert = commands
        .spawn((
            NodeBundle::default(),
            ColorSpaceNode {
                from_space: ColorSpace::Srgb,
                to_space: ColorSpace::Hsv,
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    // Threshold node
    let threshold_node = commands
        .spawn((
            NodeBundle::default(),
            ThresholdNode {
                threshold: 0.5,
                comparison: ComparisonType::Greater,
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    // Multiple probe points for analysis
    let analysis_probe = commands
        .spawn((
            NodeBundle::default(),
            PixelProbeNode {
                kernel_size: Vec2::new(3.0, 3.0),
                sample_positions: (0..10).map(|i| Vec2::new(0.1 * i as f32, 0.5)).collect(),
                interpolation: InterpolationType::Linear,
            },
            PixelDataOutput {
                pixels: Vec::new(),
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id();

    // Network output for real-time streaming
    let network_output = commands
        .spawn((
            NodeBundle::default(),
            NetworkOutputNode {
                endpoint: "ws://localhost:8080/data".to_string(),
                protocol: NetworkProtocol::WebSocket,
            },
            SinkNode,
        ))
        .id();

    // Connect the pipeline
    connect_nodes(
        &mut commands,
        camera_source,
        blur_node,
        OutputSlot::Image,
        InputSlot::Image,
    );
    connect_nodes(
        &mut commands,
        blur_node,
        color_convert,
        OutputSlot::Image,
        InputSlot::Image,
    );
    connect_nodes(
        &mut commands,
        color_convert,
        threshold_node,
        OutputSlot::Image,
        InputSlot::Image,
    );
    connect_nodes(
        &mut commands,
        threshold_node,
        analysis_probe,
        OutputSlot::Image,
        InputSlot::Image,
    );
    connect_nodes(
        &mut commands,
        analysis_probe,
        network_output,
        OutputSlot::PixelData,
        InputSlot::Buffer,
    );
}

// ✅ Conditional processing pipeline
pub fn setup_conditional_pipeline(mut commands: Commands, camera_entity: Entity) {
    let camera_source =
        spawn_camera_source_node(&mut commands, camera_entity, UVec2::new(1024, 1024));

    // Brightness analysis probe
    let brightness_probe = commands
        .spawn((
            NodeBundle::default(),
            StatisticsProbeNode {
                region: Region::FullImage,
                stats_type: StatisticsType::AverageBrightness,
            },
            BufferOutput {
                buffer: Handle::default(),
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id();

    // Conditional node (this would need custom implementation)
    let conditional_node = commands
        .spawn((
            NodeBundle::default(),
            ConditionalNode {
                condition: ConditionType::BrightnessThreshold(0.7),
                true_path: ProcessingPath::HighExposure,
                false_path: ProcessingPath::LowExposure,
            },
            TransformNode,
        ))
        .id();

    // Different processing paths based on conditions
    let high_exposure_processing = commands
        .spawn((
            NodeBundle::default(),
            ConvolutionNode {
                kernel: vec![0.0, -1.0, 0.0, -1.0, 5.0, -1.0, 0.0, -1.0, 0.0], // Sharpen
                kernel_size: UVec2::new(3, 3),
                border_mode: BorderMode::Clamp,
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    let low_exposure_processing = commands
        .spawn((
            NodeBundle::default(),
            ConvolutionNode {
                kernel: vec![
                    1.0 / 9.0,
                    1.0 / 9.0,
                    1.0 / 9.0,
                    1.0 / 9.0,
                    1.0 / 9.0,
                    1.0 / 9.0,
                    1.0 / 9.0,
                    1.0 / 9.0,
                    1.0 / 9.0,
                ], // Blur
                kernel_size: UVec2::new(3, 3),
                border_mode: BorderMode::Clamp,
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    // Connect the conditional pipeline
    connect_nodes(
        &mut commands,
        camera_source,
        brightness_probe,
        OutputSlot::Image,
        InputSlot::Image,
    );
    connect_nodes(
        &mut commands,
        brightness_probe,
        conditional_node,
        OutputSlot::Buffer,
        InputSlot::Buffer,
    );

    // These connections would be managed by the conditional node's logic
    // connect_nodes(&mut commands, conditional_node, high_exposure_processing, OutputSlot::Image, InputSlot::Image);
    // connect_nodes(&mut commands, conditional_node, low_exposure_processing, OutputSlot::Image, InputSlot::Image);
}

// ✅ Temporal analysis pipeline
pub fn setup_temporal_analysis_pipeline(mut commands: Commands, camera_entity: Entity) {
    let camera_source =
        spawn_camera_source_node(&mut commands, camera_entity, UVec2::new(1280, 720));

    // Temporal filter to smooth over time
    let temporal_filter = commands
        .spawn((
            NodeBundle::default(),
            TemporalFilterNode {
                filter_type: TemporalFilterType::ExponentialSmoothing,
                window_size: 10,
                alpha: 0.1,
                history: Vec::new(),
            },
            ImageOutput {
                image: Handle::default(),
                frame: 0,
                dirty: false,
            },
            TransformNode,
        ))
        .id();

    // Motion detection probe
    let motion_probe = commands
        .spawn((
            NodeBundle::default(),
            MotionProbeNode {
                tracking_points: vec![
                    Vec2::new(0.2, 0.2),
                    Vec2::new(0.8, 0.2),
                    Vec2::new(0.2, 0.8),
                    Vec2::new(0.8, 0.8),
                    Vec2::new(0.5, 0.5),
                ],
                temporal_window: 5,
                motion_type: MotionType::FrameDifference,
                previous_frames: Vec::new(),
            },
            BufferOutput {
                buffer: Handle::default(),
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id();

    // Statistics aggregation
    let motion_stats = commands
        .spawn((
            NodeBundle::default(),
            StatisticsProbeNode {
                region: Region::FullImage,
                stats_type: StatisticsType::MotionMagnitude,
            },
            ScalarOutput {
                value: 0.0,
                frame: 0,
                dirty: false,
            },
            ProbeNode,
        ))
        .id();

    // File output for temporal analysis
    let temporal_output = commands
        .spawn((
            NodeBundle::default(),
            FileOutputNode {
                path: "temporal_analysis.csv".to_string(),
                format: FileFormat::Csv,
            },
            SinkNode,
        ))
        .id();

    // Connect temporal pipeline
    connect_nodes(
        &mut commands,
        camera_source,
        temporal_filter,
        OutputSlot::Image,
        InputSlot::Image,
    );
    connect_nodes(
        &mut commands,
        temporal_filter,
        motion_probe,
        OutputSlot::Image,
        InputSlot::Image,
    );
    connect_nodes(
        &mut commands,
        motion_probe,
        motion_stats,
        OutputSlot::Buffer,
        InputSlot::Buffer,
    );
    connect_nodes(
        &mut commands,
        motion_stats,
        temporal_output,
        OutputSlot::Scalar,
        InputSlot::Scalar,
    );
}

// Additional component types that would be needed
#[derive(Component)]
struct ConditionalNode {
    condition: ConditionType,
    true_path: ProcessingPath,
    false_path: ProcessingPath,
}

#[derive(Clone)]
enum ConditionType {
    BrightnessThreshold(f32),
    MotionThreshold(f32),
    Custom(String),
}

#[derive(Clone)]
enum ProcessingPath {
    HighExposure,
    LowExposure,
    MotionBlur,
    NoMotion,
}

// Placeholder types for the examples
type Region = RegionType;
enum RegionType {
    FullImage,
    Rectangle(Vec2, Vec2),
    Circle(Vec2, f32),
}

type StatisticsType = StatsType;
enum StatsType {
    AverageBrightness,
    DepthHistogram,
    MotionMagnitude,
}

type ColorSpace = ColorSpaceType;
enum ColorSpaceType {
    Srgb,
    Hsv,
    Lab,
}

type MathOperation = MathOp;
enum MathOp {
    Derivative,
    Integral,
    Add,
    Multiply,
}

type MotionType = MotionTrackingType;
enum MotionTrackingType {
    OpticalFlow,
    FrameDifference,
    FeatureTracking,
}

type TemporalFilterType = TemporalFilter;
enum TemporalFilter {
    ExponentialSmoothing,
    MovingAverage,
    Kalman,
}

type FileFormat = FileFormatType;
enum FileFormatType {
    Json,
    Csv,
    Binary,
}

type NetworkProtocol = NetworkProtocolType;
enum NetworkProtocolType {
    WebSocket,
    Http,
    Udp,
}

type DepthTest = DepthTestType;
enum DepthTestType {
    Less,
    Greater,
    Equal,
}

type WindowId = u32;
impl WindowId {
    fn primary() -> Self {
        0
    }
}
