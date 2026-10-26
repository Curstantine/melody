use core_engine::SortField;
use thiserror::Error;

/// Configuration for a single column in the library table view.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnFormatConfig {
    pub header: String,
    pub format: String,
    pub width: SizeConstraint,
    pub sort_field: Option<SortField>,
}

/// Customizable formatting for the Library view.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryFormatConfig {
    pub columns: Vec<ColumnFormatConfig>,
}

impl Default for LibraryFormatConfig {
    fn default() -> Self {
        Self {
            columns: vec![
                ColumnFormatConfig {
                    header: "Title".to_string(),
                    format: "{title}".to_string(),
                    width: SizeConstraint::FillPortion(4),
                    sort_field: Some(SortField::Title),
                },
                ColumnFormatConfig {
                    header: "Artist".to_string(),
                    format: "{artist}".to_string(),
                    width: SizeConstraint::FillPortion(3),
                    sort_field: Some(SortField::Artist),
                },
                ColumnFormatConfig {
                    header: "Album".to_string(),
                    format: "{album}".to_string(),
                    width: SizeConstraint::FillPortion(3),
                    sort_field: Some(SortField::Album),
                },
                ColumnFormatConfig {
                    header: "Duration".to_string(),
                    format: "{duration_label}".to_string(),
                    width: SizeConstraint::Fixed(80.0),
                    sort_field: Some(SortField::Duration),
                },
            ],
        }
    }
}

/// Customizable formatting for Albums view (Tile and List).
#[derive(Debug, Clone, PartialEq)]
pub struct AlbumsFormatConfig {
    pub tile_line1: String,
    pub tile_line2: String,
    pub tile_line3: String,
    pub list_line1: String,
    pub list_line2: String,
    pub list_details: String,
}

impl Default for AlbumsFormatConfig {
    fn default() -> Self {
        Self {
            tile_line1: "{album}".to_string(),
            tile_line2: "{artist}[ • {year_label}]".to_string(),
            tile_line3: "{tracks_label} • {duration_label}".to_string(),
            list_line1: "{album}".to_string(),
            list_line2: "{artist}[ • {year_label}]".to_string(),
            list_details: "{tracks_label} • {duration_label}".to_string(),
        }
    }
}

/// Customizable formatting for Artists view (Tile and List).
#[derive(Debug, Clone, PartialEq)]
pub struct ArtistsFormatConfig {
    pub tile_line1: String,
    pub tile_line2: String,
    pub list_line1: String,
    pub list_line2: String,
}

impl Default for ArtistsFormatConfig {
    fn default() -> Self {
        Self {
            tile_line1: "{artist}".to_string(),
            tile_line2: "{stats_label}".to_string(),
            list_line1: "{artist}".to_string(),
            list_line2: "{stats_label}".to_string(),
        }
    }
}

/// Customizable formatting for Playlists view (Catalog Tile/List and open detail).
#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistsFormatConfig {
    pub tile_line1: String,
    pub tile_line2: String,
    pub list_line1: String,
    pub list_line2: String,
    pub list_details: String,
    pub detail_primary: String,
    pub detail_secondary: String,
    pub detail_details: String,
}

impl Default for PlaylistsFormatConfig {
    fn default() -> Self {
        Self {
            tile_line1: "{name}".to_string(),
            tile_line2: "{tracks_label}".to_string(),
            list_line1: "{name}".to_string(),
            list_line2: "{file_path}".to_string(),
            list_details: "{tracks_label}".to_string(),
            detail_primary: "{title}".to_string(),
            detail_secondary: "{artist}".to_string(),
            detail_details: "{duration_label}".to_string(),
        }
    }
}

/// Customizable formatting for Queues.
#[derive(Debug, Clone, PartialEq)]
pub struct QueueFormatConfig {
    pub track_primary: String,
    pub track_secondary: String,
    pub track_details: String,
    pub chip_format: String,
}

impl Default for QueueFormatConfig {
    fn default() -> Self {
        Self {
            track_primary: "{title}".to_string(),
            track_secondary: "{artist}".to_string(),
            track_details: "{duration_label}".to_string(),
            chip_format: "{name} ({track_count})".to_string(),
        }
    }
}

/// Aggregated view data formatting options.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ViewFormatsConfig {
    pub library: LibraryFormatConfig,
    pub albums: AlbumsFormatConfig,
    pub artists: ArtistsFormatConfig,
    pub playlists: PlaylistsFormatConfig,
    pub queue: QueueFormatConfig,
}

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
    pub view_formats: ViewFormatsConfig,
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
            view_formats: ViewFormatsConfig::default(),
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

        let mut view_formats = ViewFormatsConfig::default();
        let mut layout_nodes = Vec::new();

        for child in children_doc.nodes() {
            if child.name().value() == "view-formats" || child.name().value() == "formats" {
                view_formats = parse_view_formats_node(child);
            } else {
                layout_nodes.push(child.clone());
            }
        }

        let children = parse_nodes(&layout_nodes)?;

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
            view_formats,
        })
    } else {
        // Document without top-level `layout` block: treat entire document as column
        let mut view_formats = ViewFormatsConfig::default();
        let mut layout_nodes = Vec::new();

        for child in doc.nodes() {
            if child.name().value() == "view-formats" || child.name().value() == "formats" {
                view_formats = parse_view_formats_node(child);
            } else {
                layout_nodes.push(child.clone());
            }
        }

        let children = parse_nodes(&layout_nodes)?;
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
            view_formats,
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

fn parse_view_formats_node(node: &kdl::KdlNode) -> ViewFormatsConfig {
    let mut config = ViewFormatsConfig::default();
    let Some(children) = node.children() else {
        return config;
    };

    for child in children.nodes() {
        match child.name().value() {
            "library" => {
                if let Some(col_nodes) = child.children() {
                    let mut cols = Vec::new();
                    for col_node in col_nodes.nodes() {
                        if col_node.name().value() == "column" {
                            let header = col_node
                                .get("header")
                                .and_then(|v| v.as_string())
                                .or_else(|| get_arg(col_node, 0).and_then(|v| v.as_string()))
                                .unwrap_or("Column")
                                .to_string();
                            let format = col_node
                                .get("format")
                                .and_then(|v| v.as_string())
                                .or_else(|| get_arg(col_node, 1).and_then(|v| v.as_string()))
                                .unwrap_or("{title}")
                                .to_string();
                            let width = parse_size_prop(col_node, "width")
                                .or_else(|| {
                                    col_node
                                        .get("fill-portion")
                                        .or_else(|| col_node.get("portion"))
                                        .and_then(|v| v.as_i64())
                                        .map(|p| SizeConstraint::FillPortion(p as u16))
                                })
                                .unwrap_or(SizeConstraint::FillPortion(1));
                            let sort_field = col_node
                                .get("sort")
                                .or_else(|| col_node.get("sort-field"))
                                .and_then(|v| v.as_string())
                                .and_then(parse_sort_field_str);
                            cols.push(ColumnFormatConfig {
                                header,
                                format,
                                width,
                                sort_field,
                            });
                        }
                    }
                    if !cols.is_empty() {
                        config.library.columns = cols;
                    }
                }
            }
            "albums" => {
                if let Some(nodes) = child.children() {
                    for n in nodes.nodes() {
                        match n.name().value() {
                            "tile" => {
                                if let Some(l1) = n.get("line1").and_then(|v| v.as_string()) {
                                    config.albums.tile_line1 = l1.to_string();
                                }
                                if let Some(l2) = n.get("line2").and_then(|v| v.as_string()) {
                                    config.albums.tile_line2 = l2.to_string();
                                }
                                if let Some(l3) = n.get("line3").and_then(|v| v.as_string()) {
                                    config.albums.tile_line3 = l3.to_string();
                                }
                            }
                            "list" => {
                                if let Some(l1) = n.get("line1").and_then(|v| v.as_string()) {
                                    config.albums.list_line1 = l1.to_string();
                                }
                                if let Some(l2) = n.get("line2").and_then(|v| v.as_string()) {
                                    config.albums.list_line2 = l2.to_string();
                                }
                                if let Some(det) = n.get("details").and_then(|v| v.as_string()) {
                                    config.albums.list_details = det.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "artists" => {
                if let Some(nodes) = child.children() {
                    for n in nodes.nodes() {
                        match n.name().value() {
                            "tile" => {
                                if let Some(l1) = n.get("line1").and_then(|v| v.as_string()) {
                                    config.artists.tile_line1 = l1.to_string();
                                }
                                if let Some(l2) = n.get("line2").and_then(|v| v.as_string()) {
                                    config.artists.tile_line2 = l2.to_string();
                                }
                            }
                            "list" => {
                                if let Some(l1) = n.get("line1").and_then(|v| v.as_string()) {
                                    config.artists.list_line1 = l1.to_string();
                                }
                                if let Some(l2) = n.get("line2").and_then(|v| v.as_string()) {
                                    config.artists.list_line2 = l2.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "playlists" => {
                if let Some(nodes) = child.children() {
                    for n in nodes.nodes() {
                        match n.name().value() {
                            "tile" => {
                                if let Some(l1) = n.get("line1").and_then(|v| v.as_string()) {
                                    config.playlists.tile_line1 = l1.to_string();
                                }
                                if let Some(l2) = n.get("line2").and_then(|v| v.as_string()) {
                                    config.playlists.tile_line2 = l2.to_string();
                                }
                            }
                            "list" => {
                                if let Some(l1) = n.get("line1").and_then(|v| v.as_string()) {
                                    config.playlists.list_line1 = l1.to_string();
                                }
                                if let Some(l2) = n.get("line2").and_then(|v| v.as_string()) {
                                    config.playlists.list_line2 = l2.to_string();
                                }
                                if let Some(det) = n.get("details").and_then(|v| v.as_string()) {
                                    config.playlists.list_details = det.to_string();
                                }
                            }
                            "detail" => {
                                if let Some(prim) = n.get("primary").and_then(|v| v.as_string()) {
                                    config.playlists.detail_primary = prim.to_string();
                                }
                                if let Some(sec) = n.get("secondary").and_then(|v| v.as_string()) {
                                    config.playlists.detail_secondary = sec.to_string();
                                }
                                if let Some(det) = n.get("details").and_then(|v| v.as_string()) {
                                    config.playlists.detail_details = det.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            "queue" => {
                if let Some(nodes) = child.children() {
                    for n in nodes.nodes() {
                        match n.name().value() {
                            "track" => {
                                if let Some(prim) = n.get("primary").and_then(|v| v.as_string()) {
                                    config.queue.track_primary = prim.to_string();
                                }
                                if let Some(sec) = n.get("secondary").and_then(|v| v.as_string()) {
                                    config.queue.track_secondary = sec.to_string();
                                }
                                if let Some(det) = n.get("details").and_then(|v| v.as_string()) {
                                    config.queue.track_details = det.to_string();
                                }
                            }
                            "chip" => {
                                if let Some(fmt) = n
                                    .get("format")
                                    .and_then(|v| v.as_string())
                                    .or_else(|| get_arg(n, 0).and_then(|v| v.as_string()))
                                {
                                    config.queue.chip_format = fmt.to_string();
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    config
}

fn parse_sort_field_str(s: &str) -> Option<SortField> {
    match s.to_ascii_lowercase().as_str() {
        "title" => Some(SortField::Title),
        "artist" => Some(SortField::Artist),
        "album" => Some(SortField::Album),
        "duration" => Some(SortField::Duration),
        "track_num" | "track-num" | "track" => Some(SortField::TrackNum),
        "added_date" | "added-date" | "date" => Some(SortField::AddedDate),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_default_layout() {
        let kdl = r#"
            layout name="Default" description="Standard layout" {
                column {
                    navbar height=32
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
                                assert_eq!(col_children.len(), 1);
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

    #[test]
    fn test_parse_view_formats() {
        let kdl = r#"
            layout name="Custom" {
                view-formats {
                    library {
                        column "Track" format="{title}" fill-portion=5 sort="title"
                        column "Artist" format="{artist}" fill-portion=3 sort="artist"
                        column "Time" format="{duration_label}" width=75 sort="duration"
                    }
                    albums {
                        tile line1="{album}" line2="{artist} ({year})" line3="{tracks_label}"
                        list line1="{album}" line2="{artist}" details="{duration_label}"
                    }
                    artists {
                        tile line1="{artist}" line2="{stats_label}"
                    }
                    playlists {
                        detail primary="{title}" secondary="{artist} • {album}" details="{duration_label}"
                    }
                    queue {
                        chip format="{name} ({track_count} songs)"
                    }
                }
                column {
                    main-view fill=true
                }
            }
        "#;

        let template = parse_kdl_layout(kdl).expect("valid kdl with formats");
        assert_eq!(template.view_formats.library.columns.len(), 3);
        assert_eq!(template.view_formats.library.columns[0].header, "Track");
        assert_eq!(
            template.view_formats.library.columns[0].sort_field,
            Some(SortField::Title)
        );
        assert_eq!(template.view_formats.albums.tile_line2, "{artist} ({year})");
        assert_eq!(
            template.view_formats.albums.list_details,
            "{duration_label}"
        );
        assert_eq!(
            template.view_formats.playlists.detail_secondary,
            "{artist} • {album}"
        );
        assert_eq!(
            template.view_formats.queue.chip_format,
            "{name} ({track_count} songs)"
        );
    }
}
