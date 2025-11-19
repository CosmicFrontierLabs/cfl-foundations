use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;
use yew::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum PatternConfig {
    Check {
        checker_size: u32,
    },
    Usaf,
    Static {
        pixel_size: u32,
    },
    Pixel,
    April,
    CirclingPixel {
        orbit_count: u32,
        orbit_radius_percent: u32,
    },
    Uniform {
        level: u8,
    },
    WigglingGaussian {
        fwhm: f64,
        wiggle_radius: f64,
        intensity: f64,
    },
    PixelGrid {
        spacing: u32,
    },
    SiemensStar {
        spokes: u32,
    },
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self::April
    }
}

impl PatternConfig {
    fn name(&self) -> &'static str {
        match self {
            Self::April => "AprilTag Array",
            Self::Check { .. } => "Checkerboard",
            Self::Usaf => "USAF-1951 Target",
            Self::Static { .. } => "Digital Static",
            Self::Pixel => "Center Pixel",
            Self::CirclingPixel { .. } => "Circling Pixel",
            Self::Uniform { .. } => "Uniform Screen",
            Self::WigglingGaussian { .. } => "Wiggling Gaussian",
            Self::PixelGrid { .. } => "Pixel Grid",
            Self::SiemensStar { .. } => "Siemens Star",
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct CalibrateFrontendProps {
    pub width: u32,
    pub height: u32,
}

pub struct CalibrateFrontend {
    pattern: PatternConfig,
    invert: bool,
    current_pattern_name: String,
    image_url: String,
    image_refresh_handle: Option<gloo_timers::callback::Interval>,
    image_failure_count: u32,
}

pub enum Msg {
    UpdatePattern(PatternConfig),
    ToggleInvert,
    ApplyPattern,
    RefreshImage,
    ImageLoaded,
    ImageError,
    ResetImageInterval,
}

impl Component for CalibrateFrontend {
    type Message = Msg;
    type Properties = CalibrateFrontendProps;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let handle = gloo_timers::callback::Interval::new(100, move || {
            link.send_message(Msg::RefreshImage);
        });

        Self {
            pattern: PatternConfig::default(),
            invert: false,
            current_pattern_name: "AprilTag Array".to_string(),
            image_url: format!("/jpeg?t={}", js_sys::Date::now()),
            image_refresh_handle: Some(handle),
            image_failure_count: 0,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::UpdatePattern(pattern) => {
                self.pattern = pattern;
                ctx.link().send_message(Msg::ApplyPattern);
                true
            }
            Msg::ToggleInvert => {
                self.invert = !self.invert;
                ctx.link().send_message(Msg::ApplyPattern);
                true
            }
            Msg::ApplyPattern => {
                let pattern = self.pattern.clone();
                let invert = self.invert;
                self.current_pattern_name = pattern.name().to_string();

                wasm_bindgen_futures::spawn_local(async move {
                    let body = serde_json::json!({
                        "pattern": pattern,
                        "invert": invert,
                    });

                    let _ = Request::post("/config").json(&body).unwrap().send().await;
                });
                true
            }
            Msg::RefreshImage => {
                let link = ctx.link().clone();
                let url = format!("/jpeg?t={}", js_sys::Date::now());
                let url_clone = url.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match Request::get(&url_clone).send().await {
                        Ok(response) if response.ok() => {
                            link.send_message(Msg::ImageLoaded);
                        }
                        _ => {
                            link.send_message(Msg::ImageError);
                        }
                    }
                });
                self.image_url = url;
                false
            }
            Msg::ImageLoaded => {
                self.image_failure_count = 0;
                ctx.link().send_message(Msg::ResetImageInterval);
                false
            }
            Msg::ImageError => {
                self.image_failure_count += 1;
                ctx.link().send_message(Msg::ResetImageInterval);
                false
            }
            Msg::ResetImageInterval => {
                let delay = Self::calculate_backoff_delay(self.image_failure_count, 100, 10000);
                let link = ctx.link().clone();
                self.image_refresh_handle = None;
                self.image_refresh_handle =
                    Some(gloo_timers::callback::Interval::new(delay, move || {
                        link.send_message(Msg::RefreshImage);
                    }));
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();

        html! {
            <>
                <div class="column left-panel">
                    <h2>{"Pattern Selection"}</h2>
                    { self.view_pattern_selector(ctx) }

                    <h2 style="margin-top: 20px;">{"Pattern Parameters"}</h2>
                    { self.view_pattern_params(ctx) }

                    <div class="control-group">
                        <label style="cursor: pointer;">
                            <input
                                type="checkbox"
                                checked={self.invert}
                                onchange={ctx.link().callback(|_| Msg::ToggleInvert)}
                            />
                            <span style="margin-left: 5px;">{"Invert Colors"}</span>
                        </label>
                    </div>
                </div>

                <div class="column center-panel">
                    <div class="image-container">
                        <img
                            class="image-frame"
                            src={self.image_url.clone()}
                            alt="Calibration Pattern"
                        />
                    </div>
                </div>

                <div class="column right-panel">
                    <h2>{"Pattern Info"}</h2>
                    <div class="info-item">
                        <span class="info-label">{"Resolution:"}</span><br/>
                        {format!("{}x{}", props.width, props.height)}
                    </div>
                    <div class="info-item">
                        <span class="info-label">{"Current Pattern:"}</span><br/>
                        <span class="status">{&self.current_pattern_name}</span>
                    </div>

                    <h2 style="margin-top: 30px;">{"Endpoints"}</h2>
                    <div class="info-item">
                        <a href="/jpeg">{"JPEG Pattern"}</a><br/>
                        <a href="/config">{"Config (JSON)"}</a>
                    </div>

                    <h2 style="margin-top: 30px;">{"Info"}</h2>
                    <div class="info-item" style="font-size: 0.8em; color: #00aa00;">
                        {"This server generates calibration patterns for display testing and camera calibration."}
                        <br/><br/>
                        {"Adjust parameters in real-time using the controls on the left."}
                        <br/><br/>
                        {"Animated patterns (Static, Circling Pixel, Wiggling Gaussian) will continuously regenerate."}
                    </div>
                </div>
            </>
        }
    }

    fn destroy(&mut self, _ctx: &Context<Self>) {
        self.image_refresh_handle = None;
    }
}

impl CalibrateFrontend {
    fn calculate_backoff_delay(failure_count: u32, base_delay: u32, max_delay: u32) -> u32 {
        if failure_count == 0 {
            base_delay
        } else {
            let exponential_delay = base_delay * 2_u32.pow(failure_count.min(10));
            exponential_delay.min(max_delay)
        }
    }

    fn view_pattern_selector(&self, ctx: &Context<Self>) -> Html {
        let onchange = ctx.link().callback(|e: Event| {
            let target: HtmlInputElement = e.target_unchecked_into();
            let value = target.value();

            let pattern = match value.as_str() {
                "April" => PatternConfig::April,
                "Check" => PatternConfig::Check { checker_size: 100 },
                "Usaf" => PatternConfig::Usaf,
                "Static" => PatternConfig::Static { pixel_size: 1 },
                "Pixel" => PatternConfig::Pixel,
                "CirclingPixel" => PatternConfig::CirclingPixel {
                    orbit_count: 1,
                    orbit_radius_percent: 50,
                },
                "Uniform" => PatternConfig::Uniform { level: 128 },
                "WigglingGaussian" => PatternConfig::WigglingGaussian {
                    fwhm: 47.0,
                    wiggle_radius: 3.0,
                    intensity: 255.0,
                },
                "PixelGrid" => PatternConfig::PixelGrid { spacing: 50 },
                "SiemensStar" => PatternConfig::SiemensStar { spokes: 24 },
                _ => PatternConfig::April,
            };

            Msg::UpdatePattern(pattern)
        });

        html! {
            <div class="control-group">
                <label class="control-label">{"Pattern Type:"}</label>
                <select id="pattern-type" {onchange}>
                    <option value="April" selected={matches!(self.pattern, PatternConfig::April)}>{"AprilTag Array"}</option>
                    <option value="Check" selected={matches!(self.pattern, PatternConfig::Check {..})}>{"Checkerboard"}</option>
                    <option value="Usaf" selected={matches!(self.pattern, PatternConfig::Usaf)}>{"USAF-1951 Target"}</option>
                    <option value="Static" selected={matches!(self.pattern, PatternConfig::Static {..})}>{"Digital Static"}</option>
                    <option value="Pixel" selected={matches!(self.pattern, PatternConfig::Pixel)}>{"Center Pixel"}</option>
                    <option value="CirclingPixel" selected={matches!(self.pattern, PatternConfig::CirclingPixel {..})}>{"Circling Pixel"}</option>
                    <option value="Uniform" selected={matches!(self.pattern, PatternConfig::Uniform {..})}>{"Uniform Screen"}</option>
                    <option value="WigglingGaussian" selected={matches!(self.pattern, PatternConfig::WigglingGaussian {..})}>{"Wiggling Gaussian"}</option>
                    <option value="PixelGrid" selected={matches!(self.pattern, PatternConfig::PixelGrid {..})}>{"Pixel Grid"}</option>
                    <option value="SiemensStar" selected={matches!(self.pattern, PatternConfig::SiemensStar {..})}>{"Siemens Star"}</option>
                </select>
            </div>
        }
    }

    fn view_pattern_params(&self, ctx: &Context<Self>) -> Html {
        match &self.pattern {
            PatternConfig::Check { checker_size } => {
                let oninput = ctx.link().callback(|e: InputEvent| {
                    let target: HtmlInputElement = e.target_unchecked_into();
                    let value = target.value().parse().unwrap_or(100);
                    Msg::UpdatePattern(PatternConfig::Check {
                        checker_size: value,
                    })
                });

                html! {
                    <div class="control-group">
                        <label class="control-label">
                            {"Checker Size (px): "}
                            <span class="range-value">{checker_size}</span>
                        </label>
                        <input
                            type="range"
                            min="10"
                            max="500"
                            value={checker_size.to_string()}
                            step="10"
                            {oninput}
                        />
                    </div>
                }
            }
            PatternConfig::Static { pixel_size } => {
                let oninput = ctx.link().callback(|e: InputEvent| {
                    let target: HtmlInputElement = e.target_unchecked_into();
                    let value = target.value().parse().unwrap_or(1);
                    Msg::UpdatePattern(PatternConfig::Static { pixel_size: value })
                });

                html! {
                    <div class="control-group">
                        <label class="control-label">
                            {"Pixel Size (px): "}
                            <span class="range-value">{pixel_size}</span>
                        </label>
                        <input
                            type="range"
                            min="1"
                            max="20"
                            value={pixel_size.to_string()}
                            {oninput}
                        />
                    </div>
                }
            }
            PatternConfig::Uniform { level } => {
                let oninput = ctx.link().callback(|e: InputEvent| {
                    let target: HtmlInputElement = e.target_unchecked_into();
                    let value = target.value().parse().unwrap_or(128);
                    Msg::UpdatePattern(PatternConfig::Uniform { level: value })
                });

                html! {
                    <div class="control-group">
                        <label class="control-label">
                            {"Brightness Level: "}
                            <span class="range-value">{level}</span>
                        </label>
                        <input
                            type="range"
                            min="0"
                            max="255"
                            value={level.to_string()}
                            {oninput}
                        />
                    </div>
                }
            }
            PatternConfig::PixelGrid { spacing } => {
                let oninput = ctx.link().callback(|e: InputEvent| {
                    let target: HtmlInputElement = e.target_unchecked_into();
                    let value = target.value().parse().unwrap_or(50);
                    Msg::UpdatePattern(PatternConfig::PixelGrid { spacing: value })
                });

                html! {
                    <div class="control-group">
                        <label class="control-label">
                            {"Grid Spacing (px): "}
                            <span class="range-value">{spacing}</span>
                        </label>
                        <input
                            type="range"
                            min="10"
                            max="200"
                            value={spacing.to_string()}
                            {oninput}
                        />
                    </div>
                }
            }
            PatternConfig::SiemensStar { spokes } => {
                let oninput = ctx.link().callback(|e: InputEvent| {
                    let target: HtmlInputElement = e.target_unchecked_into();
                    let value = target.value().parse().unwrap_or(24);
                    Msg::UpdatePattern(PatternConfig::SiemensStar { spokes: value })
                });

                html! {
                    <div class="control-group">
                        <label class="control-label">
                            {"Number of Spokes: "}
                            <span class="range-value">{spokes}</span>
                        </label>
                        <input
                            type="range"
                            min="4"
                            max="72"
                            value={spokes.to_string()}
                            step="4"
                            {oninput}
                        />
                    </div>
                }
            }
            _ => html! {},
        }
    }
}
