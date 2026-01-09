use wesl::Wesl;

fn main() {
    let resolver = Wesl::new("src/probe/pipeline/shaders");

    resolver.build_artefact("kernel_small", "kernel_small");
    resolver.build_artefact("kernel_medium", "kernel_medium");
    resolver.build_artefact("kernel_large", "kernel_large");
}
