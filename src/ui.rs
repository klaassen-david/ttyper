use super::test::{results, Test};

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use results::Fraction;
use std::iter;
use tui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Span, Spans, Text},
    widgets::{Axis, Block, BorderType, Borders, Chart, Dataset, GraphType, Paragraph, Widget},
};

#[derive(Clone)]
struct SizedBlock<'a> {
    block: Block<'a>,
    area: Rect,
}

impl SizedBlock<'_> {
    fn render(self, buf: &mut Buffer) {
        self.block.render(self.area, buf)
    }
}

trait UsedWidget {}
impl UsedWidget for Paragraph<'_> {}

trait DrawInner<T> {
    fn draw_inner(&self, content: T, buf: &mut Buffer);
}

impl DrawInner<&Spans<'_>> for SizedBlock<'_> {
    fn draw_inner(&self, content: &Spans, buf: &mut Buffer) {
        let inner = self.block.inner(self.area);
        buf.set_spans(inner.x, inner.y, content, inner.width);
    }
}

impl<T> DrawInner<T> for SizedBlock<'_>
where
    T: Widget + UsedWidget,
{
    fn draw_inner(&self, content: T, buf: &mut Buffer) {
        let inner = self.block.inner(self.area);
        content.render(inner, buf);
    }
}

impl Widget for &Test {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Styles
        let title_style = Style::default()
            .patch(Style::default().add_modifier(Modifier::BOLD))
            .patch(Style::default().fg(Color::White));

        // Chunks
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Length(6)])
            .split(area);

        // Sections
        let input = SizedBlock {
            block: Block::default()
                .title(Spans::from(vec![Span::styled("Input", title_style)]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
            area: chunks[0],
        };
        input.draw_inner(
            &Spans::from(self.words[self.current_word].progress.clone()),
            buf,
        );
        input.render(buf);

        let target_lines: Vec<Spans> = {
            let progress_ind = self.words[self.current_word]
                .progress
                .len()
                .min(self.words[self.current_word].text.len());
            let words = iter::empty::<Vec<Span>>()
                .chain(self.words[..self.current_word].iter().map(|w| {
                    vec![Span::styled(
                        w.text.clone() + " ",
                        Style::default().fg(match w.progress == w.text {
                            true => Color::Green,
                            false => Color::Red,
                        }),
                    )]
                }))
                .chain(iter::once(vec![
                    Span::styled(
                        self.words[self.current_word]
                            .text
                            .chars()
                            .take(progress_ind)
                            .collect::<String>(),
                        Style::default()
                            .fg(
                                match self.words[self.current_word]
                                    .text
                                    .starts_with(&self.words[self.current_word].progress[..])
                                {
                                    true => Color::Green,
                                    false => Color::Red,
                                },
                            )
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        self.words[self.current_word]
                            .text
                            .chars()
                            .skip(progress_ind)
                            .collect::<String>()
                            + " ",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]))
                .chain(self.words[self.current_word + 1..].iter().map(|w| {
                    vec![Span::styled(
                        w.text.clone() + " ",
                        Style::default().fg(Color::Gray),
                    )]
                }));

            let mut lines: Vec<Spans> = Vec::new();
            let mut current_line: Vec<Span> = Vec::new();
            let mut current_width = 0;
            for word in words {
                let word_width: usize = word.iter().map(|s| s.width()).sum();

                if current_width + word_width > chunks[1].width as usize - 2 {
                    current_line.push(Span::raw("\n"));
                    lines.push(Spans::from(current_line.clone()));
                    current_line.clear();
                    current_width = 0;
                }

                current_line.extend(word);
                current_width += word_width;
            }
            lines.push(Spans::from(current_line));

            lines
        };
        let target = Paragraph::new(target_lines).block(
            Block::default()
                .title(Spans::from(vec![Span::styled("Prompt", title_style)]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Green)),
        );
        target.render(chunks[1], buf);
    }
}

impl Widget for &results::Results {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Styles
        let title_style = Style::default()
            .patch(Style::default().add_modifier(Modifier::BOLD))
            .patch(Style::default().fg(Color::White));

        // Chunks
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);
        let res_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1) // Graph looks tremendously better with just a little margin
            .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
            .split(chunks[0]);

        let exit = Span::styled(
            "Press any key to finish or 'r' for a another test.",
            Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::ITALIC),
        );
        buf.set_span(chunks[1].x, chunks[1].y, &exit, chunks[1].width);

        // Sections
        let mut info_text = Text::styled(
            "",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        );

        info_text.extend(vec![
            Spans::from(format!(
                "Adjusted WPM: {:.1}",
                self.cps.overall * 12f64 * f64::from(self.accuracy.overall)
            )),
            Spans::from(format!(
                "Accuracy: {:.1}%",
                f64::from(self.accuracy.overall) * 100f64
            )),
            Spans::from(format!("Raw WPM: {:.1}", self.cps.overall * 12f64)),
            Spans::from(format!("Correct Keypresses: {}", self.accuracy.overall)),
        ]);

        let mut worst_keys: Vec<(&KeyEvent, &Fraction)> = self
            .accuracy
            .per_key
            .iter()
            .filter(|(key, _)| matches!(key.code, KeyCode::Char(_)))
            .collect();
        worst_keys.sort_unstable_by_key(|x| x.1);

        info_text.extend(iter::once(Spans::from("Worst Keys:")));
        info_text.extend(
            worst_keys
                .iter()
                .take(5)
                .filter_map(|(key, acc)| {
                    if let KeyCode::Char(character) = key.code {
                        Some(format!(
                            "- {} at {:.1}% accuracy",
                            character,
                            f64::from(**acc) * 100.0,
                        ))
                    } else {
                        None
                    }
                })
                .map(Spans::from),
        );

        let info = Paragraph::new(info_text).block(
            Block::default()
                .title(Spans::from(vec![Span::styled("Results", title_style)]))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        );
        info.render(res_chunks[0], buf);

        // Raw WPM over time Graph
        let mut raw_wpm_data_over_time: Vec<(f64, f64)> = Vec::new();
        let mut min_raw_wpm = f64::MAX;
        let mut max_raw_wpm = f64::MIN;

        // Calculate wpm over time
        for time_step in 1..self.cps.per_event.len() {
            let mut cur_overall_wpm = self.cps.per_event[..time_step].len() as f64
                / self.cps.per_event[..time_step].iter().sum::<f64>();
            cur_overall_wpm *= 12f64;
            raw_wpm_data_over_time.push((time_step as f64, cur_overall_wpm));

            // Update current min and max's
            min_raw_wpm = min_raw_wpm.min(cur_overall_wpm);
            max_raw_wpm = max_raw_wpm.max(cur_overall_wpm);
        }

        let mut wpm_label_strs: Vec<String> = Vec::new();
        for wpm_step in ((min_raw_wpm as u8)..(max_raw_wpm as u8)).step_by(5) {
            let label = &format!("{}", wpm_step);
            wpm_label_strs.push(label.clone());
        }

        let wpm_datasets = vec![Dataset::default()
            .name("Raw WPM")
            .marker(Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&raw_wpm_data_over_time)];

        let wpm_chart = Chart::new(wpm_datasets)
            .block(Block::default().title("Chart"))
            .x_axis(
                Axis::default()
                    .title(Span::styled("Time", Style::default().fg(Color::Cyan)))
                    .bounds([0.0, self.cps.per_event.len() as f64]),
            )
            .y_axis(
                Axis::default()
                    .title(Span::styled("WPM", Style::default().fg(Color::Gray)))
                    .bounds([min_raw_wpm, max_raw_wpm])
                    .labels(wpm_label_strs.iter().cloned().map(Span::from).collect()),
            );
        wpm_chart.render(res_chunks[1], buf);
    }
}
