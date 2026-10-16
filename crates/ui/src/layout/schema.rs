use thiserror::Error;

/// Sizing constraint for layout containers and widgets, mapping to [`iced::Length`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SizeConstraint {
    /// Expand to take available space.
    Fill,
    /// Proportional flex factor among sibling elements.
    FillPortion(u16),
    /// Fixed pixel size.
    Fixed(f32),
    /// Shrink wrap content.
    #[default]
    Shrink,
}

impl SizeConstraint {
    pub fn to_iced_length(self) -> iced::Length {
        match self {
            Self::Fill => iced::Length::Fill,
            Self::FillPortion(p) => iced::Length::FillPortion(p),
            Self::Fixed(px) => iced::Length::Fixed(px),
            Self::Shrink => iced::Length::Shrink,
        }
    }
}

/// Direction orientation for navigation bars, dividers, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// Alignment inside a container or box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
}

impl Alignment {
    pub fn to_iced_alignment(self) -> iced::Alignment {
        match self {
            Self::Start => iced::Alignment::Start,
            Self::Center => iced::Alignment::Center,
            Self::End => iced::Alignment::End,
        }
    }
}

/// Visual style for containers and panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContainerStyle {
    #[default]
    Transparent,
    Surface,
    MainBackground,
    Panel,
}

/// Identifies a specific UI component to render.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutComponent {
    /// Navigation tab buttons + settings icon.
    Navbar {
        orientation: Orientation,
        show_settings: bool,
    },
    /// The currently selected view tab (Tracks, Lyrics, Albums, Artists, Playlists, or Settings).
    MainView,
    /// Dedicated tracks library table/grid view.
    Tracks,
    /// Dedicated albums catalog view.
    Albums,
    /// Dedicated artists catalog view.
    Artists,
    /// Dedicated playlists catalog view.
    Playlists,
    /// Dedicated queue panel.
    Queue,
    /// Dedicated now playing lyrics & cover view.
    Lyrics,
    /// Dedicated settings view.
    Settings,
    /// Full unified bottom player bar.
    PlayerBar,
    /// Mini track info: cover art + marquee title & artist.
    TrackInfo,
    /// Playback buttons: prev, play/pause, stop, next.
    PlayerControls,
    /// Position slider and duration elapsed/total.
    SeekBar,
    /// Volume icon, slider, and percentage.
    VolumeControl,
    /// Shuffle and repeat toggle buttons.
    PlaybackOptions,
    /// Bottom status bar with track count, scanning status, etc.
    StatusBar,
}

/// A node in the layout tree.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutNode {
    /// Vertical container of children.
    Column {
        children: Vec<LayoutNode>,
        width: SizeConstraint,
        height: SizeConstraint,
        spacing: f32,
        padding: [f32; 4],
        align: Alignment,
    },
    /// Horizontal container of children.
    Row {
        children: Vec<LayoutNode>,
        width: SizeConstraint,
        height: SizeConstraint,
        spacing: f32,
        padding: [f32; 4],
        align: Alignment,
    },
    /// Stack of children layered on top of each other.
    Stack {
        children: Vec<LayoutNode>,
        width: SizeConstraint,
        height: SizeConstraint,
    },
    /// Container wrapping a single child with padding and styling.
    Container {
        child: Box<LayoutNode>,
        width: SizeConstraint,
        height: SizeConstraint,
        padding: [f32; 4],
        style: ContainerStyle,
    },
    /// Separator line.
    Divider { orientation: Orientation, size: f32 },
    /// Empty space.
    Spacer {
        width: SizeConstraint,
        height: SizeConstraint,
    },
    /// Concrete UI component.
    Component(LayoutComponent),
}

/// A complete layout template definition parsed from KDL.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutTemplate {
    pub name: String,
    pub description: String,
    pub author: String,
    pub root: LayoutNode,
}

impl Default for LayoutTemplate {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            description: "Standard layout with top navigation bar and bottom player".to_string(),
            author: "Melody Team".to_string(),
            root: LayoutNode::Column {
                children: vec![
                    LayoutNode::Component(LayoutComponent::Navbar {
                        orientation: Orientation::Horizontal,
                        show_settings: true,
                    }),
                    LayoutNode::Component(LayoutComponent::MainView),
                    LayoutNode::Component(LayoutComponent::PlayerBar),
                    LayoutNode::Component(LayoutComponent::StatusBar),
                ],
                width: SizeConstraint::Fill,
                height: SizeConstraint::Fill,
                spacing: 0.0,
                padding: [0.0; 4],
                align: Alignment::Start,
            },
        }
    }
}

#[derive(Debug, Error)]
pub enum LayoutParseError {
    #[error("KDL syntax error: {0}")]
    Kdl(#[from] kdl::KdlError),
    #[error("Missing layout root node")]
    MissingLayoutNode,
    #[error("Invalid property value for '{key}': {message}")]
    InvalidProperty { key: String, message: String },
    #[error("Unknown layout node '{0}'")]
    UnknownNode(String),
}

/// Parses a KDL string into a [`LayoutTemplate`].
pub fn parse_kdl_layout(content: &str) -> Result<LayoutTemplate, LayoutParseError> {
    let doc: kdl::KdlDocument = content.parse()?;

    // Find the 'layout' node or treat root as container
    if let Some(layout_node) = doc.get("layout") {
        let name = layout_node
            .get("name")
            .and_then(|v| v.as_string())
            .or_else(|| get_arg(layout_node, 0).and_then(|v| v.as_string()))
            .unwrap_or("Custom")
            .to_string();

        let description = layout_node
            .get("description")
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();

        let author = layout_node
            .get("author")
            .and_then(|v| v.as_string())
            .unwrap_or("")
            .to_string();

        let children_doc = layout_node
            .children()
            .ok_or(LayoutParseError::MissingLayoutNode)?;

        let children = parse_nodes(children_doc.nodes())?;

        let root = if children.len() == 1 {
            children.into_iter().next().unwrap()
        } else {
            LayoutNode::Column {
                children,
                width: SizeConstraint::Fill,
                height: SizeConstraint::Fill,
                spacing: 0.0,
                padding: [0.0; 4],
                align: Alignment::Start,
            }
        };

        Ok(LayoutTemplate {
            name,
            description,
            author,
            root,
        })
    } else {
        // Document without top-level `layout` block: treat entire document as column
        let children = parse_nodes(doc.nodes())?;
        let root = if children.len() == 1 {
            children.into_iter().next().unwrap()
        } else {
            LayoutNode::Column {
                children,
                width: SizeConstraint::Fill,
                height: SizeConstraint::Fill,
                spacing: 0.0,
                padding: [0.0; 4],
                align: Alignment::Start,
            }
        };

        Ok(LayoutTemplate {
            name: "Custom".to_string(),
            description: String::new(),
            author: String::new(),
            root,
        })
    }
}

fn parse_nodes(nodes: &[kdl::KdlNode]) -> Result<Vec<LayoutNode>, LayoutParseError> {
    let mut result = Vec::new();
    for node in nodes {
        if let Some(layout_node) = parse_single_node(node)? {
            result.push(layout_node);
        }
    }
    Ok(result)
}

fn parse_single_node(node: &kdl::KdlNode) -> Result<Option<LayoutNode>, LayoutParseError> {
    let name = node.name().value();

    match name {
        "column" | "col" | "vertical" | "vbox" => {
            let width = parse_size_prop(node, "width").unwrap_or(SizeConstraint::Fill);
            let height = parse_size_prop(node, "height").unwrap_or(SizeConstraint::Fill);
            let spacing = parse_float_prop(node, "spacing").unwrap_or(0.0);
            let padding = parse_padding_prop(node);
            let align = parse_align_prop(node);

            let children = if let Some(children_doc) = node.children() {
                parse_nodes(children_doc.nodes())?
            } else {
                Vec::new()
            };

            Ok(Some(LayoutNode::Column {
                children,
                width,
                height,
                spacing,
                padding,
                align,
            }))
        }

        "row" | "horizontal" | "hbox" => {
            let width = parse_size_prop(node, "width").unwrap_or(SizeConstraint::Fill);
            let height = parse_size_prop(node, "height").unwrap_or(SizeConstraint::Shrink);
            let spacing = parse_float_prop(node, "spacing").unwrap_or(0.0);
            let padding = parse_padding_prop(node);
            let align = parse_align_prop(node);

            let children = if let Some(children_doc) = node.children() {
                parse_nodes(children_doc.nodes())?
            } else {
                Vec::new()
            };

            Ok(Some(LayoutNode::Row {
                children,
                width,
                height,
                spacing,
                padding,
                align,
            }))
        }

        "stack" => {
            let width = parse_size_prop(node, "width").unwrap_or(SizeConstraint::Fill);
            let height = parse_size_prop(node, "height").unwrap_or(SizeConstraint::Fill);
            let children = if let Some(children_doc) = node.children() {
                parse_nodes(children_doc.nodes())?
            } else {
                Vec::new()
            };

            Ok(Some(LayoutNode::Stack {
                children,
                width,
                height,
            }))
        }

        "container" | "pane" | "panel" => {
            let width = parse_size_prop(node, "width").unwrap_or(SizeConstraint::Fill);
            let height = parse_size_prop(node, "height").unwrap_or(SizeConstraint::Fill);
            let padding = parse_padding_prop(node);
            let style = parse_style_prop(node, name == "pane" || name == "panel");

            let children = if let Some(children_doc) = node.children() {
                parse_nodes(children_doc.nodes())?
            } else {
                Vec::new()
            };

            let child = if children.len() == 1 {
                children.into_iter().next().unwrap()
            } else {
                LayoutNode::Column {
                    children,
                    width: SizeConstraint::Fill,
                    height: SizeConstraint::Fill,
                    spacing: 0.0,
                    padding: [0.0; 4],
                    align: Alignment::Start,
                }
            };

            Ok(Some(LayoutNode::Container {
                child: Box::new(child),
                width,
                height,
                padding,
                style,
            }))
        }

        "divider" | "border" | "separator" => {
            let orientation = parse_orientation_prop(node).unwrap_or(Orientation::Horizontal);
            let size = parse_float_prop(node, "size").unwrap_or(1.0);
            Ok(Some(LayoutNode::Divider { orientation, size }))
        }

        "spacer" | "space" => {
            let width = parse_size_prop(node, "width").unwrap_or_else(|| {
                if parse_bool_prop(node, "fill").unwrap_or(false) {
                    SizeConstraint::Fill
                } else {
                    SizeConstraint::Shrink
                }
            });
            let height = parse_size_prop(node, "height").unwrap_or_else(|| {
                if parse_bool_prop(node, "fill").unwrap_or(false) {
                    SizeConstraint::Fill
                } else {
                    SizeConstraint::Shrink
                }
            });
            Ok(Some(LayoutNode::Spacer { width, height }))
        }

        // Components
        "navbar" | "tabs" | "nav" => {
            let orientation = parse_orientation_prop(node).unwrap_or(Orientation::Horizontal);
            let show_settings = parse_bool_prop(node, "show_settings").unwrap_or(true);
            Ok(Some(LayoutNode::Component(LayoutComponent::Navbar {
                orientation,
                show_settings,
            })))
        }

        "main-view" | "main_view" | "main" | "tab-content" | "active-tab" => {
            Ok(Some(LayoutNode::Component(LayoutComponent::MainView)))
        }

        "tracks" | "library" | "songs" => Ok(Some(LayoutNode::Component(LayoutComponent::Tracks))),
        "albums" => Ok(Some(LayoutNode::Component(LayoutComponent::Albums))),
        "artists" => Ok(Some(LayoutNode::Component(LayoutComponent::Artists))),
        "playlists" => Ok(Some(LayoutNode::Component(LayoutComponent::Playlists))),
        "queue" | "queues" => Ok(Some(LayoutNode::Component(LayoutComponent::Queue))),
        "lyrics" | "now-playing" | "now_playing" => {
            Ok(Some(LayoutNode::Component(LayoutComponent::Lyrics)))
        }
        "settings" => Ok(Some(LayoutNode::Component(LayoutComponent::Settings))),

        "player-bar" | "player_bar" | "player" => {
            Ok(Some(LayoutNode::Component(LayoutComponent::PlayerBar)))
        }
        "track-info" | "track_info" => Ok(Some(LayoutNode::Component(LayoutComponent::TrackInfo))),
        "player-controls" | "controls" => {
            Ok(Some(LayoutNode::Component(LayoutComponent::PlayerControls)))
        }
        "seek-bar" | "seek_bar" | "seek" => {
            Ok(Some(LayoutNode::Component(LayoutComponent::SeekBar)))
        }
        "volume-control" | "volume_control" | "volume" => {
            Ok(Some(LayoutNode::Component(LayoutComponent::VolumeControl)))
        }
        "playback-options" | "playback_options" | "options" => Ok(Some(LayoutNode::Component(
            LayoutComponent::PlaybackOptions,
        ))),

        "status-bar" | "status_bar" | "status" => {
            Ok(Some(LayoutNode::Component(LayoutComponent::StatusBar)))
        }

        other => {
            tracing::warn!("Ignoring unknown layout node: {other}");
            Ok(None)
        }
    }
}

fn parse_size_prop(node: &kdl::KdlNode, prop_name: &str) -> Option<SizeConstraint> {
    // Check specific property: e.g. width=240, width="fill", width="shrink", fill=true
    if let Some(val) = node.get(prop_name) {
        if let Some(s) = val.as_string() {
            return match s.to_lowercase().as_str() {
                "fill" => Some(SizeConstraint::Fill),
                "shrink" => Some(SizeConstraint::Shrink),
                s if s.starts_with("fill:") => {
                    let portion = s
                        .strip_prefix("fill:")
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(1);
                    Some(SizeConstraint::FillPortion(portion))
                }
                s => s
                    .trim_end_matches("px")
                    .parse::<f32>()
                    .ok()
                    .map(SizeConstraint::Fixed),
            };
        }
        if let Some(i) = val.as_integer() {
            return Some(SizeConstraint::Fixed(i as f32));
        }
        if let Some(f) = val.as_float() {
            return Some(SizeConstraint::Fixed(f as f32));
        }
        if let Some(b) = val.as_bool() {
            return if b {
                Some(SizeConstraint::Fill)
            } else {
                Some(SizeConstraint::Shrink)
            };
        }
    }

    // Check generic `fill=true` or `fill=2`
    if let Some(fill_val) = node.get("fill") {
        if let Some(b) = fill_val.as_bool() {
            if b {
                return Some(SizeConstraint::Fill);
            }
        }
        if let Some(i) = fill_val.as_integer() {
            if i > 0 {
                return Some(SizeConstraint::FillPortion(i as u16));
            }
        }
    }

    None
}

fn parse_float_prop(node: &kdl::KdlNode, prop_name: &str) -> Option<f32> {
    node.get(prop_name).and_then(|v| {
        v.as_float()
            .map(|f| f as f32)
            .or_else(|| v.as_integer().map(|i| i as f32))
            .or_else(|| v.as_string().and_then(|s| s.parse::<f32>().ok()))
    })
}

fn parse_bool_prop(node: &kdl::KdlNode, prop_name: &str) -> Option<bool> {
    node.get(prop_name).and_then(|v| {
        v.as_bool()
            .or_else(|| v.as_string().and_then(|s| s.parse::<bool>().ok()))
    })
}

fn get_arg(node: &kdl::KdlNode, index: usize) -> Option<&kdl::KdlValue> {
    node.entries()
        .iter()
        .filter(|e| e.name().is_none())
        .nth(index)
        .map(|e| e.value())
}

fn parse_orientation_prop(node: &kdl::KdlNode) -> Option<Orientation> {
    node.get("orientation")
        .or_else(|| get_arg(node, 0))
        .and_then(|v| v.as_string())
        .and_then(|s| match s.to_lowercase().as_str() {
            "horizontal" | "horiz" | "h" => Some(Orientation::Horizontal),
            "vertical" | "vert" | "v" => Some(Orientation::Vertical),
            _ => None,
        })
}

fn parse_align_prop(node: &kdl::KdlNode) -> Alignment {
    node.get("align")
        .and_then(|v| v.as_string())
        .map(|s| match s.to_lowercase().as_str() {
            "center" => Alignment::Center,
            "end" | "right" | "bottom" => Alignment::End,
            _ => Alignment::Start,
        })
        .unwrap_or(Alignment::Start)
}

fn parse_padding_prop(node: &kdl::KdlNode) -> [f32; 4] {
    if let Some(val) = node.get("padding") {
        if let Some(i) = val.as_integer() {
            let p = i as f32;
            return [p, p, p, p];
        }
        if let Some(f) = val.as_float() {
            let p = f as f32;
            return [p, p, p, p];
        }
        if let Some(s) = val.as_string() {
            let parts: Vec<f32> = s
                .split_whitespace()
                .filter_map(|p| p.parse::<f32>().ok())
                .collect();
            match parts.len() {
                1 => return [parts[0], parts[0], parts[0], parts[0]],
                2 => return [parts[0], parts[1], parts[0], parts[1]],
                4 => return [parts[0], parts[1], parts[2], parts[3]],
                _ => {}
            }
        }
    }
    [0.0; 4]
}

fn parse_style_prop(node: &kdl::KdlNode, default_panel: bool) -> ContainerStyle {
    node.get("style")
        .and_then(|v| v.as_string())
        .map(|s| match s.to_lowercase().as_str() {
            "surface" => ContainerStyle::Surface,
            "background" | "bg" => ContainerStyle::MainBackground,
            "panel" | "pane" => ContainerStyle::Panel,
            _ => ContainerStyle::Transparent,
        })
        .unwrap_or(if default_panel {
            ContainerStyle::Panel
        } else {
            ContainerStyle::Transparent
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_layout() {
        let kdl = r#"
            layout name="Default" description="Standard layout" {
                column {
                    navbar height=33
                    main-view fill=true
                    player-bar height=72
                    status-bar height=24
                }
            }
        "#;

        let template = parse_kdl_layout(kdl).expect("valid layout");
        assert_eq!(template.name, "Default");
        assert_eq!(template.description, "Standard layout");

        match template.root {
            LayoutNode::Column { children, .. } => {
                assert_eq!(children.len(), 4);
                assert_eq!(
                    children[0],
                    LayoutNode::Component(LayoutComponent::Navbar {
                        orientation: Orientation::Horizontal,
                        show_settings: true,
                    })
                );
                assert_eq!(
                    children[1],
                    LayoutNode::Component(LayoutComponent::MainView)
                );
                assert_eq!(
                    children[2],
                    LayoutNode::Component(LayoutComponent::PlayerBar)
                );
                assert_eq!(
                    children[3],
                    LayoutNode::Component(LayoutComponent::StatusBar)
                );
            }
            other => panic!("expected column root, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_sidebar_layout() {
        let kdl = r#"
            layout name="Sidebar" {
                column {
                    row fill=true {
                        column width=200 {
                            navbar orientation="vertical" fill=true
                            divider orientation="horizontal"
                            track-info height=64
                        }
                        divider orientation="vertical"
                        column fill=true {
                            main-view fill=true
                        }
                    }
                    divider orientation="horizontal"
                    player-bar height=72
                    status-bar height=24
                }
            }
        "#;

        let template = parse_kdl_layout(kdl).expect("valid sidebar layout");
        assert_eq!(template.name, "Sidebar");

        match template.root {
            LayoutNode::Column { children, .. } => {
                assert_eq!(children.len(), 4);
                match &children[0] {
                    LayoutNode::Row {
                        children: row_children,
                        ..
                    } => {
                        assert_eq!(row_children.len(), 3);
                        match &row_children[0] {
                            LayoutNode::Column {
                                width,
                                children: col_children,
                                ..
                            } => {
                                assert_eq!(*width, SizeConstraint::Fixed(200.0));
                                assert_eq!(col_children.len(), 3);
                                assert_eq!(
                                    col_children[0],
                                    LayoutNode::Component(LayoutComponent::Navbar {
                                        orientation: Orientation::Vertical,
                                        show_settings: true,
                                    })
                                );
                            }
                            other => panic!("expected column, got {:?}", other),
                        }
                    }
                    other => panic!("expected row, got {:?}", other),
                }
            }
            other => panic!("expected column, got {:?}", other),
        }
    }

    #[test]
    fn test_size_constraint_parsing() {
        let kdl = r#"
            column {
                spacer width=150 height="fill"
                spacer width="shrink" height="fill:2"
            }
        "#;

        let template = parse_kdl_layout(kdl).expect("valid kdl");
        match template.root {
            LayoutNode::Column { children, .. } => {
                assert_eq!(
                    children[0],
                    LayoutNode::Spacer {
                        width: SizeConstraint::Fixed(150.0),
                        height: SizeConstraint::Fill,
                    }
                );
                assert_eq!(
                    children[1],
                    LayoutNode::Spacer {
                        width: SizeConstraint::Shrink,
                        height: SizeConstraint::FillPortion(2),
                    }
                );
            }
            other => panic!("expected column, got {:?}", other),
        }
    }
}
