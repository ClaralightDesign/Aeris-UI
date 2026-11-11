//! 由卡片组成的单选列表。

use canopus_target_private::LvxEventCallback;

use super::{
    cards::Choice,
    lv::{self, Obj},
};

pub struct ChoiceList<const N: usize> {
    rows: [Option<Choice>; N],
}

impl<const N: usize> ChoiceList<N> {
    pub const EMPTY: Self = Self { rows: [None; N] };

    pub unsafe fn new<'a>(
        parent: Obj,
        top: i32,
        step: i32,
        options: impl IntoIterator<Item = (&'a str, &'a str)>,
        selected: Option<usize>,
        on_select: LvxEventCallback,
    ) -> Self {
        let mut list = Self::EMPTY;
        for (index, (title, subtitle)) in options.into_iter().take(N).enumerate() {
            let card = unsafe { lv::card(parent, 4, top + index as i32 * step) };
            if !card.is_null() {
                list.rows[index] =
                    Some(unsafe { Choice::radio(card, title, subtitle, selected == Some(index)) });
                lv::bind(card, on_select, index);
            }
        }
        list
    }

    pub fn select(&mut self, index: usize) {
        for (i, choice) in self.iter_mut() {
            choice.set_selected(i == index);
        }
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut Choice)> {
        self.rows
            .iter_mut()
            .enumerate()
            .filter_map(|(index, choice)| choice.as_mut().map(|choice| (index, choice)))
    }
}
