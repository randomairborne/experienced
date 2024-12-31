use std::thread::JoinHandle;

use xpd_rank_card::{customizations::Customizations, *};

const VALK_PFP: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAQAAAAEABAMAAACuXLVVAAAAIGNIUk0AAHomAACAhAAA+gAAAIDoAAB1MAAA6mAAADqYAAAXcJy6UTwAAAAYUExURXG0zgAAAFdXV6ampoaGhr6zpHxfQ2VPOt35dJcAAAABYktHRAH/Ai3eAAAAB3RJTUUH5wMDFSE5W/eo1AAAAQtJREFUeNrt1NENgjAUQFFXYAVWYAVXcAVXYH0hoQlpSqGY2Dae82WE9971x8cDAAAAAAAAAAAAAAAAAADgR4aNAAEC/jNgPTwuBAgQ8J8B69FpI0CAgL4DhozczLgjQICAPgPCkSkjtXg/I0CAgD4Dzg4PJ8YEAQIE9BEQLyg5cEWYFyBAQHsBVxcPN8U7BAgQ0FbAlcNhcLohjkn+egECBFQPKPE8cXpQgAABzQXkwsIfUElwblaAAAF9BeyP3Z396rgAAQJ+EvCqTIAAAfUD3pUJECCgvYB5kfp89N28yR3J7RQgQED9gPjhfmG8/Oh56r1UYOpdAQIEtBFwtLBUyY7wrgABAqoHfABW2cbX3ElRgQAAACV0RVh0ZGF0ZTpjcmVhdGUAMjAyMy0wMy0wM1QyMTozMzo1NiswMDowMNpnAp0AAAAldEVYdGRhdGU6bW9kaWZ5ADIwMjMtMDMtMDNUMjE6MzM6NTYrMDA6MDCrOrohAAAAKHRFWHRkYXRlOnRpbWVzdGFtcAAyMDIzLTAzLTAzVDIxOjMzOjU3KzAwOjAwWliQSgAAAABJRU5ErkJggg==";

fn main() {
    std::fs::remove_dir_all("rendered-cards").ok();
    std::fs::create_dir_all("rendered-cards").unwrap();
    render_classic_l().unwrap();
    render_classic_r().unwrap();
    render_vertical().unwrap();
    render_vertical_procedural();
}

fn new_state() -> SvgState {
    SvgState::new("xpd-card-resources").unwrap()
}

fn render_classic_l() -> Result<(), Error> {
    let state = new_state();
    let xp = 49;
    let customizations = Customizations {
        toy: Some("bee.png".to_string()),
        ..state.customizations_for("classic.svg").unwrap().clone()
    };
    let context = Context {
        level: 1,
        rank: 1,
        name: "Testy McTestington".to_string(),
        percentage: xp,
        current: xp,
        needed: 100 - xp,
        customizations,
        avatar: VALK_PFP.to_string(),
    };
    let output = state.sync_render(&context)?;
    std::fs::write("rendered-cards/renderer_test_classic_l.png", output).unwrap();
    Ok(())
}

fn render_classic_r() -> Result<(), Error> {
    let state = new_state();
    let xp = 51;
    let customizations = Customizations {
        toy: Some("cow.png".to_string()),
        ..state.customizations_for("classic.svg").unwrap().clone()
    };
    let context = Context {
        level: 1,
        rank: 1,
        name: "Testy McTestington".to_string(),
        percentage: xp,
        current: xp,
        needed: 100 - xp,
        customizations,
        avatar: VALK_PFP.to_string(),
    };
    let output = state.sync_render(&context)?;
    std::fs::write("rendered-cards/renderer_test_classic_r.png", output).unwrap();
    Ok(())
}

fn render_vertical() -> Result<(), Error> {
    let state = new_state();
    let xp = 99;
    let customizations = Customizations {
        font: "Montserrat-Alt1".to_string(),
        ..state.customizations_for("vertical.svg").unwrap().clone()
    };
    let context = Context {
        level: 420,
        rank: 100_000,
        name: "Testy McTestington".to_string(),
        percentage: xp,
        current: xp,
        needed: 100 - xp,
        customizations,
        avatar: VALK_PFP.to_string(),
    };
    let svg = state.render_svg(&context)?;
    let png = state.sync_render(&context)?;
    std::fs::write("rendered-cards/renderer_test_vertical.svg", svg).unwrap();
    std::fs::write("rendered-cards/renderer_test_vertical.png", png).unwrap();
    Ok(())
}

fn render_vertical_procedural() {
    let mut handles: Vec<JoinHandle<()>> = Vec::with_capacity(100);
    std::fs::create_dir_all("rendered-cards/test-procedural/").unwrap();
    for xp in (1..=100).step_by(2) {
        let spawn = std::thread::spawn(move || {
            let state = new_state();
            let context = Context {
                level: 69,
                rank: 1_000_000,
                name: "Testy McTestington".to_string(),
                percentage: xp,
                current: xp,
                needed: 100 - xp,
                customizations: state.customizations_for("vertical.svg").unwrap().clone(),
                avatar: VALK_PFP.to_string(),
            };
            let output = state.sync_render(&context).unwrap();
            std::fs::write(
                format!("rendered-cards/test-procedural/renderer_test_vertical_{xp:0>3}xp.png"),
                output,
            )
            .unwrap();
        });
        handles.push(spawn);
    }
    for handle in handles {
        handle.join().unwrap();
    }
}
