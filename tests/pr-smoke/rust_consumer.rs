#[path = "track.rs"]
mod model;

fn main() {
    let id = model::TrackId::new(42).expect("valid bounded ID");
    assert_eq!(id.get(), 42);
    assert!(model::TrackId::new(0).is_none());
    assert!(model::TrackId::new(65536).is_none());
    println!("RUST SMOKE PASSED");
}