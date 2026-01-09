use std::{f32::consts::PI, marker::PhantomData};

use bevy::{
    color::palettes::tailwind::*, ecs::query::{QueryData, QueryFilter}, picking::{hover::PickingInteraction, pointer::PointerInteraction}, prelude::*
};

fn main() {
    App::new()
        // MeshPickingPlugin is not a default plugin
        .add_plugins((DefaultPlugins, MeshPickingPlugin))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (draw_mesh_intersections, rotate))
        .run();
}

/// A marker component for our shapes so we can query them separately from the ground plane.
#[derive(Component)]
struct Shape;

const SHAPES_X_EXTENT: f32 = 14.0;
const EXTRUSION_X_EXTENT: f32 = 16.0;
const Z_EXTENT: f32 = 5.0;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Set up the materials.
    let white_matl = materials.add(Color::WHITE);
    let ground_matl = materials.add(Color::from(GRAY_300));
    let hover_matl = materials.add(Color::from(CYAN_300));
    let pressed_matl = materials.add(Color::from(YELLOW_300));

    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Tetrahedron::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Cone::default()),
        meshes.add(ConicalFrustum::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
    ];

    let extrusions = [
        meshes.add(Extrusion::new(Rectangle::default(), 1.)),
        meshes.add(Extrusion::new(Capsule2d::default(), 1.)),
        meshes.add(Extrusion::new(Annulus::default(), 1.)),
        meshes.add(Extrusion::new(Circle::default(), 1.)),
        meshes.add(Extrusion::new(Ellipse::default(), 1.)),
        meshes.add(Extrusion::new(RegularPolygon::default(), 1.)),
        meshes.add(Extrusion::new(Triangle2d::default(), 1.)),
    ];

    let num_shapes = shapes.len();

    // Spawn the shapes. The meshes will be pickable by default.
    for (i, shape) in shapes.into_iter().enumerate() {
        commands
            .spawn((
                Mesh3d(shape),
                MeshMaterial3d(white_matl.clone()),
                Transform::from_xyz(
                    -SHAPES_X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * SHAPES_X_EXTENT,
                    2.0,
                    Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
            ))
            .observe(update_material_on::<Pointer<Over>>(hover_matl.clone()))
            .observe(update_material_on::<Pointer<Out>>(white_matl.clone()))
            .observe(update_material_on::<Pointer<Press>>(pressed_matl.clone()))
            .observe(update_material_on::<Pointer<Release>>(hover_matl.clone()))
            .observe(rotate_on_drag);
    }

    let num_extrusions = extrusions.len();

    for (i, shape) in extrusions.into_iter().enumerate() {
        commands
            .spawn((
                Mesh3d(shape),
                MeshMaterial3d(white_matl.clone()),
                Transform::from_xyz(
                    -EXTRUSION_X_EXTENT / 2.
                        + i as f32 / (num_extrusions - 1) as f32 * EXTRUSION_X_EXTENT,
                    2.0,
                    -Z_EXTENT / 2.,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                Shape,
            ))
            .observe(update_material_on::<Pointer<Over>>(hover_matl.clone()))
            .observe(update_material_on::<Pointer<Out>>(white_matl.clone()))
            .observe(update_material_on::<Pointer<Press>>(pressed_matl.clone()))
            .observe(update_material_on::<Pointer<Release>>(hover_matl.clone()))
            .observe(rotate_on_drag);
    }

    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0).subdivisions(10))),
        MeshMaterial3d(ground_matl.clone()),
        Pickable::IGNORE, // Disable picking for the ground plane.
    ));

    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            shadow_depth_bias: 0.2,
            ..default()
        },
        Transform::from_xyz(8.0, 16.0, 8.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 7., 14.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    // Instructions
    commands.spawn((
        Text::new("Hover over the shapes to pick them\nDrag to rotate"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Returns an observer that updates the entity's material to the one specified.
fn update_material_on<E: EntityEvent>(
    new_material: Handle<StandardMaterial>,
) -> impl Fn(On<E>, Query<&mut MeshMaterial3d<StandardMaterial>>) {
    // An observer closure that captures `new_material`. We do this to avoid needing to write four
    // versions of this observer, each triggered by a different event and with a different hardcoded
    // material. Instead, the event type is a generic, and the material is passed in.
    move |trigger, mut query| {
        if let Ok(mut material) = query.get_mut(trigger.target()) {
            material.0 = new_material.clone();
        }
    }
}

/// A system that draws hit indicators for every pointer.
fn draw_mesh_intersections(pointers: Query<&PointerInteraction>, mut gizmos: Gizmos) {
    for (point, normal) in pointers
        .iter()
        .filter_map(|interaction| interaction.get_nearest_hit())
        .filter_map(|(_entity, hit)| hit.position.zip(hit.normal))
    {
        gizmos.sphere(point, 0.05, RED_500);
        gizmos.arrow(point, point + normal.normalize() * 0.5, PINK_100);
    }
}

/// A system that rotates all shapes.
fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

/// An observer to rotate an entity when it is dragged
fn rotate_on_drag(drag: On<Pointer<Drag>>, mut transforms: Query<&mut Transform>) {
    let mut transform = transforms.get_mut(drag.target()).unwrap();
    transform.rotate_y(drag.delta.x * 0.02);
    transform.rotate_x(drag.delta.y * 0.02);
}








    
pub trait Interactive:  Sized + Send + Sync + 'static + Component {
   type Query: QueryData;
   type Filter: QueryFilter;
   
//    fn spawn<T>(&mut self, bundle: T) -> EntityCommands
//    where
//        T: Bundle;
     
   fn on_pointer_move(event: On<Pointer<Move>>, query: Query<Self::Query>) {}
   fn on_pointer_enter(event: On<Pointer<Over>>, query: Query<Self::Query>) {}
   fn on_pointer_exit(event: On<Pointer<Out>>, query: Query<Self::Query>) {}
   fn on_pointer_press(event: On<Pointer<Press>>, query: Query<Self::Query>) {}
   fn on_pointer_release(event: On<Pointer<Release>>, query: Query<Self::Query>) {}
   fn on_pointer_click(event: On<Pointer<Click>>, query: Query<Self::Query>) {}
   fn on_pointer_drag(event: On<Pointer<Drag>>, query: Query<Self::Query>) {}
   fn on_pointer_scroll(event: On<Pointer<Scroll>>, query: Query<Self::Query>) {}
}






pub struct InteractivePlugin<T: Interactive> {
    _marker: PhantomData<T>,
}

impl<T: Interactive> Plugin for InteractivePlugin<T> {
    fn build(&self, app: &mut App) {
    }

}





#[derive(Component)]
#[require(Mesh3d, MeshMaterial3d<StandardMaterial>)]
pub struct FiniteElement;



#[derive(QueryData)]
pub struct FiniteElementQuery<'a>{
    entity: Entity,
    transform: &'a Transform,
    material: &'a MeshMaterial3d<StandardMaterial>,
}


impl Interactive for FiniteElement {
    type Query = FiniteElementQuery<'static>;
    type Filter = With<FiniteElement>;

    fn on_pointer_move(event: On<Pointer<Move>>, mut query: Query<Self::Query>) {
        if let Ok(mut transform) = query.get_mut(event.entity) {
           
        }
    }
    
    
}






impl<T: Interactive> InteractivePlugin<T> {
    fn on_add(commands: &mut Commands, query: Query<Entity, Added<T>>) {
        for (entity) in query.iter() {
           commands.entity(entity)
            .observe(T::on_pointer_move)
           .observe(T::on_pointer_enter)
           .observe(T::on_pointer_exit)
           .observe(T::on_pointer_press)
           .observe(T::on_pointer_release)
           .observe(T::on_pointer_click)
           .observe(T::on_pointer_drag)
           .observe(T::on_pointer_scroll);
        }
    }
}



#[derive(Message)]
pub enum FiniteElementEvent {
    Create,
}

pub struct FiniteElementPlugin;

impl FiniteElementPlugin {
    fn probe_event_handler(
        mut probe_events: MessageReader<FiniteElementEvent>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for event in probe_events.read() {
            match event {
                FiniteElementEvent::Create => {
                    FiniteElement::spawn(&mut commands, &mut meshes, &mut materials);
                }
            }
        }
    }
}


impl FiniteElement {
    fn new() -> Self {
        Self {}
    }
    fn spawn(
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<StandardMaterial>>,
    ) -> Entity {
        // Spawn a 3D object with move observer
        commands
            .spawn((
                Self::new(),
                Mesh3d(meshes.add(Cuboid::default())),
                MeshMaterial3d(materials.add(Color::srgba(0.0, 1.0, 1.0, 1.0))),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ))
            .observe(Self::on_pointer_move)
            .observe(Self::on_pointer_enter)
            .observe(Self::on_pointer_exit)
            .id()
    }
    // Move handler
    fn on_pointer_move(move_event: On<Pointer<Move>>, mut query: Query<&mut Transform>) {
        if let Ok(mut transform) = query.get_mut(move_event.target()) {
            let delta = move_event.delta;

            // Rotate the object based on pointer movement
            transform.rotate_y(delta.x * 0.005);
            transform.rotate_x(-delta.y * 0.005);
        }
    }

    // Optional: Handle enter/exit for visual feedback
    fn on_pointer_enter(
        trigger: On<Pointer<Over>>,
        mut query: Query<&mut MeshMaterial3d<StandardMaterial>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        if let Ok(mut material) = query.get_mut(trigger.target()) {
            material.0 = materials.add(Color::srgba(1.0, 1.0, 0.0, 1.0));
        }
    }

    fn on_pointer_exit(
        trigger: On<Pointer<Out>>,
        mut query: Query<&mut MeshMaterial3d<StandardMaterial>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        if let Ok(mut material) = query.get_mut(trigger.target()) {
            material.0 = materials.add(Color::srgba(0.0, 1.0, 1.0, 1.0));
        }
    }

    fn update_material_on<E: EntityEvent>(
        new_material: Handle<StandardMaterial>,
    ) -> impl Fn(On<E>, Query<&mut MeshMaterial3d<StandardMaterial>, With<Self>>) {
        // An observer closure that captures `new_material`. We do this to avoid needing to write four
        // versions of this observer, each triggered by a different event and with a different hardcoded
        // material. Instead, the event type is a generic, and the material is passed in.
        move |trigger, mut query| {
            if let Ok(mut material) = query.get_mut(trigger.target()) {
                material.0 = new_material.clone();
            }
        }
    }

    fn handle_3d_interactions(
        mut query: Query<
            (&PickingInteraction, &mut MeshMaterial3d<StandardMaterial>),
            Changed<PickingInteraction>,
        >,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for (interaction, mut material) in query.iter_mut() {
            match *interaction {
                PickingInteraction::Pressed => {
                    // Change material when pressed
                    *material = MeshMaterial3d(materials.add(Color::srgba(1.0, 1.0, 0.0, 1.0)));
                }
                PickingInteraction::Hovered => {
                    // Change material when hovered
                    *material = MeshMaterial3d(materials.add(Color::srgba(0.0, 1.0, 1.0, 1.0)));
                }
                PickingInteraction::None => {
                    // Default material
                    *material = MeshMaterial3d(materials.add(Color::srgba(1.0, 1.0, 1.0, 1.0)));
                }
            }
        }
    }
    
}
