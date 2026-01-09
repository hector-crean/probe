//! Render graph node for probe computation.

use bevy::{
    ecs::{
        query::QueryState,
        system::lifetimeless::Read,
        world::{FromWorld, World},
    },
    render::{
        render_graph::{self, RenderLabel},
        render_resource::{ComputePassDescriptor, PipelineCache},
    },
    render::renderer::RenderContext,
};

use super::bind_group::{KernelBindGroup, PreparedKernel, ProbePipeline};

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
pub struct ProbeNodeLabel;

/// The render graph node that executes the probe compute shader.
pub struct ProbeNode {
    // We query for probes that are ready to be processed.
    query: QueryState<(Read<PreparedKernel>, Read<KernelBindGroup>)>,
}

impl FromWorld for ProbeNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered::<(Read<PreparedKernel>, Read<KernelBindGroup>), ()>(),
        }
    }
}

impl render_graph::Node for ProbeNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let probe_pipeline = world.resource::<ProbePipeline>();

        for (probe, kernel_bind_group) in self.query.iter_manual(world) {
            // Choose pipeline based on kernel size
            let kernel_area =
                kernel_bind_group.settings.kernel_size.x * kernel_bind_group.settings.kernel_size.y;

            let (pipeline_id, dispatch_type) = if kernel_area <= 16.0 {
                // 4x4 or smaller - use small pipeline (1x1 workgroups)
                (probe_pipeline.small_pipeline, "small")
            } else if kernel_area <= 256.0 {
                // 5x5 to 16x16 - use medium pipeline (8x8 workgroups)
                (probe_pipeline.medium_pipeline, "medium")
            } else {
                // Larger than 16x16 - use large pipeline (16x16 workgroups)
                (probe_pipeline.large_pipeline, "large")
            };

            let Some(pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id) else {
                continue;
            };

            let mut pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("probe_compute_pass"),
                        timestamp_writes: None,
                    });

            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &probe.0, &[]);

            match dispatch_type {
                "small" => {
                    // Small pipeline uses 1x1 workgroups - dispatch one per pixel
                    pass.dispatch_workgroups(
                        kernel_bind_group.settings.kernel_size.x as u32,
                        kernel_bind_group.settings.kernel_size.y as u32,
                        1,
                    );
                }
                "medium" => {
                    // Medium pipeline uses 8x8 workgroups - calculate needed workgroups
                    let workgroups_x =
                        (kernel_bind_group.settings.kernel_size.x as u32).div_ceil(8);
                    let workgroups_y =
                        (kernel_bind_group.settings.kernel_size.y as u32).div_ceil(8);
                    pass.dispatch_workgroups(workgroups_x.max(1), workgroups_y.max(1), 1);
                }
                "large" => {
                    // Large pipeline uses 16x16 workgroups - calculate needed workgroups
                    let workgroups_x =
                        (kernel_bind_group.settings.kernel_size.x as u32).div_ceil(16);
                    let workgroups_y =
                        (kernel_bind_group.settings.kernel_size.y as u32).div_ceil(16);
                    pass.dispatch_workgroups(workgroups_x.max(1), workgroups_y.max(1), 1);
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
