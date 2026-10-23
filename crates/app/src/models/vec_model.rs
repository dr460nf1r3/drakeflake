#![expect(
    clippy::arbitrary_source_item_ordering,
    reason = "a macro can only be imported after its definition"
)]

macro_rules! vec_list_model {
    (
        $model:ident, $row:ident, $rows:ident, notify = $notify:ident
        $(, next = $next:ident)?
        $(, page = $page:ident)?
    ) => {
        impl ::qtbridge::QListModel for $model {
            type Item = $row;

            fn len(&self) -> usize {
                self.$rows.len()
            }

            fn get(&self, index: usize) -> Option<&Self::Item> {
                self.$rows.get(index)
            }

            fn set_unnotified(&mut self, index: usize, value: Self::Item) -> bool {
                self.$rows.get_mut(index).map(|row| *row = value).is_some()
            }

            fn push_unnotified(&mut self, value: Self::Item) {
                self.$rows.push(value);
            }

            fn pop_unnotified(&mut self) -> Option<Self::Item> {
                self.$rows.pop()
            }

            fn remove_unnotified(&mut self, index: usize) -> Self::Item {
                if index < self.$rows.len() {
                    self.$rows.remove(index)
                } else {
                    $row::default()
                }
            }

            fn reset_unnotified(&mut self) {
                self.$rows.clear();
                $(self.$rows.append(&mut self.$next);)?
            }
        }

        impl $model {
            fn bump(&mut self) {
                self.revision = self.revision.wrapping_add(1);
                self.$notify();
            }

            $(
                fn append_page(&mut self) {
                    for row in self.$page(self.$rows.len()) {
                        ::qtbridge::QListModelBase::push(self, row);
                    }

                    self.$notify();
                }
            )?
        }
    };
}

pub(crate) use vec_list_model;

pub fn match_rows<R>(
    matches: &[usize],
    skip: usize,
    take: usize,
    row: impl Fn(usize) -> Option<R>,
) -> Vec<R> {
    matches
        .iter()
        .skip(skip)
        .take(take)
        .filter_map(|position| row(*position))
        .collect()
}

pub fn count_arg(count: i32) -> usize {
    usize::try_from(count).unwrap_or(0)
}

pub fn top_json<R>((total, rows): (usize, Vec<R>)) -> serde_json::Value
where
    R: serde::Serialize,
{
    serde_json::json!(super::TopResult { total, rows })
}
