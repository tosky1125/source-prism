use axum::{
    extract::Path,
    response::{Html, IntoResponse},
};

const SHELL: &str = include_str!("../assets/repo_explorer.html");

pub(crate) async fn repo(Path(repo_id): Path<String>) -> impl IntoResponse {
    Html(render_shell(&repo_id, ExplorerView::Overview))
}

pub(crate) async fn repo_view(Path((repo_id, view)): Path<(String, String)>) -> impl IntoResponse {
    Html(render_shell(&repo_id, ExplorerView::from_path(&view)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExplorerView {
    Overview,
    Files,
    Symbols,
    References,
    Impact,
    Tests,
    Coverage,
    Docs,
    Search,
    Runs,
    Sync,
}

impl ExplorerView {
    const fn id(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Files => "files",
            Self::Symbols => "symbols",
            Self::References => "references",
            Self::Impact => "impact",
            Self::Tests => "tests",
            Self::Coverage => "coverage",
            Self::Docs => "docs",
            Self::Search => "search",
            Self::Runs => "runs",
            Self::Sync => "sync",
        }
    }

    fn from_path(value: &str) -> Self {
        match value {
            "files" => Self::Files,
            "symbols" => Self::Symbols,
            "references" => Self::References,
            "impact" => Self::Impact,
            "tests" => Self::Tests,
            "coverage" => Self::Coverage,
            "docs" => Self::Docs,
            "search" => Self::Search,
            "runs" => Self::Runs,
            "sync" => Self::Sync,
            _ => Self::Overview,
        }
    }
}

fn render_shell(repo_id: &str, view: ExplorerView) -> String {
    SHELL
        .replace("__REPO_ID__", html_escape(repo_id).as_str())
        .replace("__INITIAL_VIEW__", view.id())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
