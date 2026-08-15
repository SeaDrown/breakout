use bevy::{
    math::{bounding::{Aabb2d, BoundingCircle, BoundingVolume, IntersectsVolume}},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Score(0)) // setting our initial score
        .insert_resource(ClearColor(BACKGROUND_COLOUR)) // setting the background colour
        .add_systems(Startup, set_up) // add set up to start up schedule

        // chaining the systems together so they work in order
        .add_systems(Update, (apply_velocity, move_paddle, check_for_collisions).chain())

        // finally adding the update scoreboard system
        .add_systems(Update, update_scoreboard)
        .run();
}

// PADDLE SETTINGS
const PADDLE_SIZE: Vec2 = Vec2::new(200.0, 40.0); // paddle size in pixels
const GAP_BETWEEN_PADDLE_AND_FLOOR: f32 = 50.0; // how far the paddle is above the floor
const PADDLE_SPEED: f32 = 300.0; // pixels per second the paddle travels
const PADDLE_WALL_PADDING: f32 = 50.0; // how close the paddle can get to the wall
const PADDLE_COLOUR: Color = Color::srgb(0.2, 0.2, 0.6);

// BALL SETTINGS
const BALL_START_POSITION: Vec3 = Vec3::new(0., 0., 1.0); // Z component is effectively the Z-index
const BALL_RADIUS: f32 = 12.0;
const BALL_SPEED: f32 = 350.0; // pixels per second
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(1.0, -1.0); 
const BALL_COLOUR: Color = Color::srgb(0.2, 0.8, 0.2);

// WALL SETTINGS
const WALL_THICKNESS: f32 = 40.0;

const TOP_WALL_POSITION: Vec2 = Vec2::new(0.0, 300.0);
const BOTTOM_WALL_POSITION: Vec2 = Vec2::new(0.0, -300.0);
const LEFT_WALL_POSITION: Vec2 = Vec2::new(-400.0, 0.0);
const RIGHT_WALL_POSITION: Vec2 = Vec2::new(400.0, 0.0);

const WALL_COLOUR: Color = Color::srgb(0.1, 0.1, 0.1);
const BACKGROUND_COLOUR: Color = Color::srgb(0.8, 0.8, 0.8);

// SCOREBOARD SETTINGS
const SCOREBOARD_COLOUR: Color = Color::srgb(0.0, 0.0, 0.0);
const SCORE_COLOUR: Color = Color::srgb(1.0, 0.2, 0.2);
const SCOREBOARD_FONT_SIZE: FontSize = FontSize::Px(24.0);

// COMPONENTS

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Resource)]
struct BounceSound(Handle<AudioSource>);

// gives collider a default implementation, which is necessary to be required for
// other components
#[derive(Component, Default)]
struct Collider;

#[derive(Component)]
#[require(Transform, Sprite, Collider)] // 
struct Wall;

#[derive(Event)]
struct BallCollided;

enum WallLocation {
    Top,
    Bottom,
    Left,
    Right
}

impl WallLocation {
    // get the (x, y) positions of the walls
    fn get_position(&self) -> Vec2 {
        match self {
            WallLocation::Top => TOP_WALL_POSITION,
            WallLocation::Bottom => BOTTOM_WALL_POSITION,
            WallLocation::Left => LEFT_WALL_POSITION,
            WallLocation::Right => RIGHT_WALL_POSITION
        }
    }

    // get the (x, y) size of the walls
    fn get_size(&self) -> Vec2 {
        let mut arena_dimensions: Vec2 = Vec2::new(0.0, 0.0); // this will tell us how big our arena actually is

        arena_dimensions += TOP_WALL_POSITION + RIGHT_WALL_POSITION; // adding the positive walls
        arena_dimensions -= BOTTOM_WALL_POSITION + LEFT_WALL_POSITION; // subtracting the negative walls

        // the arena dimensions must be bigger than 0 on both axis. if they are not, the arena has negative area,
        // which is wrong. in such a case, we must cancel the program, since we cannot mutate constants

        assert!(arena_dimensions.x > 0.0 && arena_dimensions.y > 0.0); // error if the arena's dimensions are negative

        match self {
            WallLocation::Top | WallLocation::Bottom => { // top wall and bottom wall have the same area
                Vec2::new(arena_dimensions.x, WALL_THICKNESS,)
            }
            WallLocation::Left | WallLocation::Right => { // same with left wall and right wall
                Vec2::new(WALL_THICKNESS, arena_dimensions.y)
            }
        }
    }
}

impl Wall {
    // for this new function, we are taking in which wall we want to spawn. we could simply
    // add a "set_position" function, but that would mean that calling new would leave the user
    // with a wall that doesn't have a position/ has a default position, which i dont think makes
    // sense for this game

    // this return a new entity with the Wall, Sprite, Transform component. the Collider component
    // is omitted, because we have a default implementation that we dont need to configure for our
    // particular use in this case
    fn new(location: WallLocation) -> (Wall, Sprite, Transform) {
        (
            Wall,
            Sprite::from_color(WALL_COLOUR, Vec2::ONE),
            Transform {
                // transforms take a Vec3, so we use extend to add a z value. also, this serves as
                // the z-index of the wall, which decided which sprites get rendered on top of each
                // other.
                translation: location.get_position().extend(0.0),

                // this is just a bevy thing, the z component of scale must be set to 1.0 for 2d
                // objects, otherwise strange behaviour occurs
                scale: location.get_size().extend(1.0),

                // implementing the rest of the attributes as default
                ..default()
            }
        )
    }
}

// this stores the score for the game. The resource trait is effectively a 'global variable' that
// we can access from any scope. The Deref and DerefMut traits allow us to dereference any reference
// to Score by using *, and dereference mutably using *mut, respectively. This is what will allow us
// to update the score directly
#[derive(Resource, Deref, DerefMut)]
struct Score(u32);

#[derive(Component)]
struct ScoreboardUI;

// the set up system, this is where we will add the entities
fn set_up(
    mut commands: Commands, // use to alter the game state (add/delete/edit entities)
    mut meshes: ResMut<Assets<Mesh>>, // idk
    mut materials: ResMut<Assets<ColorMaterial>>, // idk
    _asset_server: Res<AssetServer> // idk
) {
    // spawn camera, stardard first action for must bevy projects
    commands.spawn(Camera2d);

    // load a sound. ion have one yet so this is commented but if you find one you can add it
    // let ball_collision_sound = asset_server.load("sounds/breakout_collision.ogg");
    // commands.insert_resource(CollisionSound(ball_collision_sound));

    // calculating the paddle's y position. notice that is is not mutable, since it will not
    // change.
    let paddle_y = BOTTOM_WALL_POSITION.y + GAP_BETWEEN_PADDLE_AND_FLOOR;

    // spawning our paddle, with every component it needs to function
    commands.spawn((
        // Sprite component is the image for the paddle (in this case a block of colour)
        Sprite::from_color(PADDLE_COLOUR, Vec2::ONE),

        // Transform component is the position, rotation, size and allat stuff. we only need
        // the position and size for our purposes, everything else can be default
        Transform {
            translation: Vec3::new(0.0, paddle_y, 0.0),
            scale: PADDLE_SIZE.extend(1.0), // remember for 2d, z must equal 1.0
            ..default()
        },

        // adding the paddle and collider components
        Paddle,
        Collider
    ));

    // spawning our ball, with every component it needs to function
    commands.spawn((
        // circle shape requires a Mesh2d, and setting the colour requires a MeshMaterial2d
        Mesh2d(meshes.add(Circle::default())),
        MeshMaterial2d(materials.add(BALL_COLOUR)),

        // Transform component is the position, rotation, size and allat stuff. we only need
        // the position and size for our purposes, everything else can be default
        Transform {
            translation: BALL_START_POSITION,
            scale: Vec2::splat(BALL_RADIUS * 2.0).extend(1.0),  // remember for 2d, z must equal 1.0
            ..default()
        },

        // adding the ball component, basically a tag that tells us this component is the ball
        Ball,

        // Velocity must be in the normalised direction of travel * ball speed
        Velocity(INITIAL_BALL_DIRECTION.normalize() * BALL_SPEED)
    ));

    // spawning our scoreboard, to see our score, shocking i know
    commands.spawn((
        // text component
        Text::new("SCORE: "),

        // text font component
        TextFont {
            font_size: SCOREBOARD_FONT_SIZE,
            ..default()
        },

        // text colour component, all very riveting
        TextColor(SCOREBOARD_COLOUR),

        Node {
            position_type: PositionType::Absolute,
            ..default()
        },

        // this macro defines children of the component?
        children![
            (
                TextSpan::default(),
                TextFont {
                    font_size: SCOREBOARD_FONT_SIZE,
                    ..default()
                },
                TextColor(SCORE_COLOUR)
            )
        ],

        // adding the ScoreboardUI component which acts like a tag that tells us this is the Scoreboard
        ScoreboardUI
    ));

    // spawn the walls
    commands.spawn(Wall::new(WallLocation::Top));
    commands.spawn(Wall::new(WallLocation::Bottom));
    commands.spawn(Wall::new(WallLocation::Left));
    commands.spawn(Wall::new(WallLocation::Right));
}

fn move_paddle(
    keyboard_input: Res<ButtonInput<KeyCode>>, // get the button input resource
    mut paddle_transform: Single<&mut Transform, With<Paddle>>, // look for exactly 1 entity that has
    // Paddle component, and return that entity's Transform component. will return nothing if not exactly
    // 1 entity satisfies those requirements
    time: Res<Time> // get the time resource
) {
    // stores the x velocity direction of the paddle
    let mut velocity_direction_x: f32 = 0.0;

    if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
        velocity_direction_x += 1.0; // velocity direction goes right (positive x)
    } else if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
        velocity_direction_x -= 1.0; // velocity direction goes left (negative x)
    }

    // where the paddle could end up if this projected x is not out of bounds
    // this does velocity direction * paddle speed * delta time, to normalise the speed for different frame rates
    let projected_paddle_x = paddle_transform.translation.x + (velocity_direction_x * PADDLE_SPEED * time.delta_secs());
    
    // the maximum left and right translation the paddle can go. just basic maths
    let left_bound = LEFT_WALL_POSITION.x + WALL_THICKNESS + (PADDLE_SIZE.x / 2.0);
    let right_bound = RIGHT_WALL_POSITION.x - WALL_THICKNESS - (PADDLE_SIZE.x / 2.0);

    // keeps the projected_paddle_x within the bounds above
    let new_paddle_x = projected_paddle_x.clamp(left_bound, right_bound);

    paddle_transform.translation.x = new_paddle_x;
}

// Query will retrieve the transform and velocity of any entities that have both
// this will just apply the velocity to the transform
fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        // update with delta time for proper physics and allat
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();
    }
}

// ngl im not sure what's happening here, will annotate later
fn update_scoreboard(
    score: Res<Score>,
    score_entity: Single<Entity, (With<ScoreboardUI>, With<Text>)>,
    mut writer: TextUiWriter
) {
    *writer.text(*score_entity, 1) = score.to_string();
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum CollisionFace {
    Top,
    Bottom,
    Left,
    Right
}

// this function will return Some<CollisionFace> if the ball collides with the bounding box
// btw Aabb2d == Axis Aligned Bounding Box 2 Dimensions. what a ridiculous name
fn ball_collision(ball: BoundingCircle, bounding_box: Aabb2d) -> Option<CollisionFace> {

    // if the ball does NOT intersect the bounding box, return None
    if !ball.intersects(&bounding_box) {
        return None;
    }

    // if the control flow reaches here, this means that this particular bounding box is colliding
    // with the ball.

    // getting the point in the AABB that is closest to the center of the ball, and calculating an
    // offset to find the side which it collided with
    let closest_point = bounding_box.closest_point(ball.center());
    let offset = ball.center() - closest_point; // basically a vec2 from the point to the centre

    // some logic to calculate which face the ball hit
    let side_of_collision: CollisionFace = {
        // if absolute x is bigger than absolute y, the ball must've hit either the left or right
        if offset.x.abs() > offset.y.abs() {
            if offset.x < 0.0 {
                CollisionFace::Left
            } else {
                CollisionFace::Right
            }
        } else if offset.y > 0.0 {
            CollisionFace::Top
        } else {
            CollisionFace::Bottom
        }
    };

    Some(side_of_collision)
}

fn check_for_collisions(
    mut commands: Commands,
    ball_query: Single<(&mut Velocity, &Transform), With<Ball>>,
    collider_query: Query<(Entity, &Transform), With<Collider>>,
    mut score_resource: ResMut<Score>
) {
    let (mut ball_velocity, ball_transform) = ball_query.into_inner(); 

    for (_collider_entity, collider_transform) in &collider_query {
        let collision_face: Option<CollisionFace> = ball_collision(
            BoundingCircle {
                center: ball_transform.translation.truncate(),
                circle: Circle::new(BALL_RADIUS)
            },
            Aabb2d::new(
                collider_transform.translation.truncate(),
                collider_transform.scale.truncate() / 2.0)
            );

        if let Some(collision_face) = collision_face {
            // any observers of the BallCollided event get triggered
            commands.trigger(BallCollided);

            **score_resource += 1;

            let mut reflect_x = false;
            let mut reflect_y = false;

            match collision_face {
                // if the ball hits the top face, reflect on y only if the y component of velocity
                // is downwards (negative, smaller than 0)
                CollisionFace::Top => {reflect_y = ball_velocity.y < 0.0}

                // only reflect if y comp is bigger than 0
                CollisionFace::Bottom => {reflect_y = ball_velocity.y > 0.0}

                CollisionFace::Left => {reflect_x = ball_velocity.x > 0.0}

                // only reflect if y comp is bigger than 0
                CollisionFace::Right => {reflect_x = ball_velocity.x < 0.0}
            }

            // actually implemet the reflections

            if reflect_x {
                ball_velocity.x *= -1.0;
            } else if reflect_y {
                ball_velocity.y *= -1.0;
            }
        }
    }
}
