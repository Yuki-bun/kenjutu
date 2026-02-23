use crossterm::event::KeyEvent;
use git2::Repository;
use kenjutu_types::CommitId;
use ratatui::{
    style::{Color, Modifier, Style},
    Frame,
};

use crate::screens::commit_log::CommitLogScreen;
use crate::screens::review::ReviewScreen;
use crate::screens::ScreenOutcome;

pub struct App {
    pub commit_log: CommitLogScreen,
    pub review: Option<ReviewScreen>,
    pub should_quit: bool,
    pub repository: Repository,
    pub error_message: Option<String>,
    pub local_dir: String,
}

impl App {
    pub fn new(local_dir: String, repository: Repository) -> Self {
        let commit_log = CommitLogScreen::new(local_dir.clone());
        Self {
            commit_log,
            review: None,
            should_quit: false,
            repository,
            error_message: None,
            local_dir,
        }
    }

    pub fn load_initial_commits(&mut self) {
        if let Err(e) = self.commit_log.load_commits() {
            self.error_message = Some(e.to_string());
        }
    }

    pub fn screen_name(&self) -> &'static str {
        if self.review.is_some() {
            "Review"
        } else {
            "CommitLog"
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        self.error_message = None;

        let outcome = if let Some(review) = &mut self.review {
            review.handle_key_event(key, &self.repository)
        } else {
            self.commit_log.handle_key_event(key)
        };

        match outcome {
            ScreenOutcome::Continue => {}
            ScreenOutcome::Quit => {
                self.should_quit = true;
            }
            ScreenOutcome::EnterReview(commit) => {
                let commit_id: CommitId = match commit.commit_id.parse() {
                    Ok(id) => id,
                    Err(e) => {
                        log::error!("invalid commit id '{}': {}", commit.commit_id, e);
                        self.error_message = Some(format!("Invalid commit id: {}", e));
                        return;
                    }
                };

                log::info!(
                    "entering review for commit {} ({})",
                    commit.change_id,
                    commit.commit_id
                );

                match ReviewScreen::new(commit, commit_id, &self.repository, self.local_dir.clone())
                {
                    Ok(screen) => {
                        self.review = Some(screen);
                    }
                    Err(e) => {
                        log::error!("failed to enter review: {}", e);
                        self.error_message = Some(e.to_string());
                    }
                }
            }
            ScreenOutcome::ExitReview => {
                self.review = None;
                // Refresh commit log in case the user described a commit during review
                if let Err(e) = self.commit_log.load_commits() {
                    self.error_message = Some(e.to_string());
                }
            }
            ScreenOutcome::Error(msg) => {
                self.error_message = Some(msg);
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        if let Some(review) = &mut self.review {
            review.render(frame);
        } else {
            self.commit_log.render(frame);
        }

        if let Some(ref msg) = self.error_message {
            let area = frame.area();
            let error_area = ratatui::layout::Rect {
                x: area.x,
                y: area.y + area.height.saturating_sub(2),
                width: area.width,
                height: 1.min(area.height),
            };
            let error_line = ratatui::text::Line::from(ratatui::text::Span::styled(
                format!(" Error: {} ", msg),
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ));
            frame.render_widget(error_line, error_area);
        }
    }
}
