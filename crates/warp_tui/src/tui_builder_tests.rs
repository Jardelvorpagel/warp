use pathfinder_color::ColorU;
use warp::appearance::Appearance;
use warpui_core::App;

use super::TuiUiBuilder;

/// The warping indicator's base color is the brand Lilac-200 (`#D2B5FF`).
///
/// Interim assertion: this pins the hardcoded brand value in
/// [`TuiUiBuilder::warping_base_fill`]. When the theme palette gains a `lilac`
/// token and the builder is switched to read it, update this expectation to the
/// token's resolved color.
#[test]
fn warping_base_color_is_brand_lilac_200() {
    App::test((), |mut app| async move {
        // `TuiUiBuilder` reads theme colors from the `Appearance` singleton.
        app.update(|ctx| {
            ctx.add_singleton_model(|_| Appearance::mock());
        });
        app.read(|app_ctx| {
            let builder = TuiUiBuilder::from_app(app_ctx);
            assert_eq!(builder.warping_base_color(), ColorU::from_u32(0xD2B5FFFF));
        });
    });
}
