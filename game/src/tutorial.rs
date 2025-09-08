use crate::game::GameState;
use crate::goals::*;
use serde::{Deserialize, Serialize};
use starling::prelude::Nanotime;

#[derive(Debug, Clone)]
pub struct Tutorial {
    pub chapters: Vec<TutorialChapter>,
    pub current: usize,
}

impl Tutorial {
    fn from_storage(chapters: Vec<TutorialChapterFileStorage>) -> Self {
        Self {
            chapters: chapters
                .into_iter()
                .map(|t| TutorialChapter::from_storage(t))
                .collect(),
            current: 0,
        }
    }

    pub fn current(&self) -> Option<&TutorialChapter> {
        self.chapters.get(self.current)
    }

    pub fn update(&mut self, state: &GameState) -> bool {
        let mut any_chapter_completed = false;
        if let Some(chapter) = self.chapters.get_mut(self.current) {
            let before = chapter.is_complete;
            for cond in &mut chapter.conditions {
                cond.update(state);
            }
            chapter.is_complete = chapter.is_complete();
            let after = chapter.is_complete;
            any_chapter_completed |= !before && after;
        }
        any_chapter_completed
    }

    pub fn is_complete(&self) -> bool {
        self.chapters.iter().all(|c| c.is_complete())
    }

    pub fn next(&mut self) {
        if self.current + 1 < self.chapters.len() {
            self.current += 1;
        }
    }

    pub fn prev(&mut self) {
        if self.current > 0 {
            self.current -= 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct TutorialChapter {
    pub title: String,
    pub intro: String,
    pub conditions: Vec<Goal>,
    pub ending: String,
    pub is_complete: bool,
}

impl TutorialChapter {
    fn from_storage(chapter: TutorialChapterFileStorage) -> Self {
        Self {
            title: chapter.title,
            intro: chapter.intro,
            conditions: chapter
                .conditions
                .iter()
                .map(|s| {
                    let mut g = Goal::new(s.cond);
                    g.is_permanent = s.is_permanent;
                    if s.seconds > 0 {
                        let t = Nanotime::secs(s.seconds.into());
                        g.dur = Some(GoalDuration {
                            required: t,
                            actual: Nanotime::ZERO,
                        });
                    }
                    g
                })
                .collect(),
            ending: chapter.ending,
            is_complete: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.is_complete | self.conditions.iter().all(|g| g.is_complete)
    }
}

impl TutorialChapter {
    pub fn new(
        title: impl Into<String>,
        intro: impl Into<String>,
        conditions: &[(GoalCondition, bool)],
        ending: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            intro: intro.into(),
            conditions: conditions
                .iter()
                .map(|(c, is_permanent)| {
                    let mut g = Goal::new(*c);
                    g.is_permanent = *is_permanent;
                    g
                })
                .collect(),
            ending: ending.into(),
            is_complete: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TutorialChapterFileStorage {
    title: String,
    intro: String,
    conditions: Vec<ConditionFileStorage>,
    ending: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConditionFileStorage {
    cond: GoalCondition,
    is_permanent: bool,
    seconds: u16,
}

pub fn load_tutorial_from_file(
    path: &std::path::Path,
) -> Result<Tutorial, Box<dyn std::error::Error>> {
    let s = std::fs::read_to_string(path)?;
    let storage = serde_yaml::from_str::<Vec<TutorialChapterFileStorage>>(&s)?;
    let tutorial = Tutorial::from_storage(storage);
    Ok(tutorial)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_to_file() {
        let t = load_tutorial_from_file(std::path::Path::new("../assets/tutorial.yaml")).unwrap();
        println!("{:#?}", t);
    }
}
