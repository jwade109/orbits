use self::super::Item;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ItemFilter {
    Any,
    Solid,
    Liquid,
    Mineable,
    Pellets,
    Foodstuffs,
    Item(Item),
    Many(Vec<Item>),
}

impl ItemFilter {
    pub fn passes(&self, item: Item) -> bool {
        match self {
            ItemFilter::Any => true,
            ItemFilter::Liquid => item.is_fluid(),
            ItemFilter::Solid => item.is_solid_cargo(),
            ItemFilter::Mineable => item.is_mineable(),
            ItemFilter::Pellets => item.is_pellet(),
            ItemFilter::Foodstuffs => todo!(),
            ItemFilter::Item(filt) => item == *filt,
            ItemFilter::Many(items) => items.contains(&item),
        }
    }
}
