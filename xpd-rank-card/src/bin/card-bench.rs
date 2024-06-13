use std::time::Instant;

use xpd_rank_card::{customizations::Customizations, Context, SvgState};

const VALK_PFP: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAQAAAAEABAMAAACuXLVVAAAAIGNIUk0AAHomAACAhAAA+gAAAIDoAAB1MAAA6mAAADqYAAAXcJy6UTwAAAAYUExURXG0zgAAAFdXV6ampoaGhr6zpHxfQ2VPOt35dJcAAAABYktHRAH/Ai3eAAAAB3RJTUUH5wMDFSE5W/eo1AAAAQtJREFUeNrt1NENgjAUQFFXYAVWYAVXcAVXYH0hoQlpSqGY2Dae82WE9971x8cDAAAAAAAAAAAAAAAAAADgR4aNAAEC/jNgPTwuBAgQ8J8B69FpI0CAgL4DhozczLgjQICAPgPCkSkjtXg/I0CAgD4Dzg4PJ8YEAQIE9BEQLyg5cEWYFyBAQHsBVxcPN8U7BAgQ0FbAlcNhcLohjkn+egECBFQPKPE8cXpQgAABzQXkwsIfUElwblaAAAF9BeyP3Z396rgAAQJ+EvCqTIAAAfUD3pUJECCgvYB5kfp89N28yR3J7RQgQED9gPjhfmG8/Oh56r1UYOpdAQIEtBFwtLBUyY7wrgABAqoHfABW2cbX3ElRgQAAACV0RVh0ZGF0ZTpjcmVhdGUAMjAyMy0wMy0wM1QyMTozMzo1NiswMDowMNpnAp0AAAAldEVYdGRhdGU6bW9kaWZ5ADIwMjMtMDMtMDNUMjE6MzM6NTYrMDA6MDCrOrohAAAAKHRFWHRkYXRlOnRpbWVzdGFtcAAyMDIzLTAzLTAzVDIxOjMzOjU3KzAwOjAwWliQSgAAAABJRU5ErkJggg==";

fn main() {
    let state = SvgState::new("xpd-card-resources").unwrap();
    let context = Context {
        level: 694,
        rank: 124,
        name: "Testy McTestington".to_string(),
        percentage: 30,
        current: 124,
        needed: 213,
        customizations: Customizations::default(),
        avatar: VALK_PFP.to_string(),
    };
    let mut total = 0.0;
    let times = 10000;
    for _ in 0..times {
        let start = Instant::now();
        let data = state.sync_render(&context).unwrap();
        total += start.elapsed().as_secs_f64();
        std::fs::write("/dev/null", data).unwrap();
    }
    let time_per_card = total / times as f64;
    println!("Took {total} seconds to render the card {times} times ({time_per_card}s / card)");
}
