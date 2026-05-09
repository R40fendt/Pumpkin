use pumpkin_data::item_stack::ItemStack;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::position::BlockPos;
use std::{
    any::Any,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::sync::Mutex;

use crate::{
    block::entities::BlockEntity,
    inventory::{Clearable, Inventory, InventoryFuture},
};
use crate::block::entities::PropertyDelegate;

pub struct LecternBlockEntity {
    pub position: BlockPos,
    pub book: Arc<Mutex<ItemStack>>,
    pub dirty: AtomicBool,
    pub page: i32
}

impl BlockEntity for LecternBlockEntity {
    fn resource_location(&self) -> &'static str {
        Self::ID
    }

    fn get_position(&self) -> BlockPos {
        self.position
    }

    fn from_nbt(nbt: &NbtCompound, position: BlockPos) -> Self
    where
        Self: Sized,
    {
        let book = nbt
            .get_compound("Book")
            .and_then(ItemStack::read_item_stack)
            .map(|stack| Arc::new(Mutex::new(stack)))
            .unwrap_or_else(|| Arc::new(Mutex::new(ItemStack::EMPTY.clone())));
        let page=nbt.get_int("Page").unwrap_or(0);

        Self {
            position,
            book,
            dirty: AtomicBool::new(false),
            page
        }
    }

    fn write_nbt<'a>(
        &'a self,
        nbt: &'a mut NbtCompound,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
        Box::pin(async move {
            let book = self.book.lock().await;
            if !book.is_empty() {
                let mut book_nbt = NbtCompound::default();
                book.write_item_stack(&mut book_nbt);
                nbt.put_compound("Book", book_nbt);
            }
        })
    }

    fn get_inventory(self: Arc<Self>) -> Option<Arc<dyn Inventory>> {
        Some(self)
    }

    fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Relaxed)
    }

    fn clear_dirty(&self) {
        self.dirty.store(false, Ordering::Relaxed);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn to_property_delegate(self: Arc<Self>) -> Option<Arc<dyn PropertyDelegate>> {
        Some(self as Arc<dyn PropertyDelegate>)
    }
}

impl LecternBlockEntity {
    pub const ID: &'static str = "minecraft:lectern";

    #[must_use]
    pub fn new(position: BlockPos) -> Self {
        Self {
            position,
            book: Arc::new(Mutex::new(ItemStack::EMPTY.clone())),
            dirty: AtomicBool::new(false),
            page: 0
        }
    }

}

impl Inventory for LecternBlockEntity {
    fn size(&self) -> usize {
        1
    }

    fn is_empty(&self) -> InventoryFuture<'_, bool> {
        Box::pin(async move { self.book.lock().await.is_empty() })
    }

    fn get_stack(&self, _slot: usize) -> InventoryFuture<'_, Arc<Mutex<ItemStack>>> {
        Box::pin(async move { self.book.clone() })
    }

    fn remove_stack(&self, _slot: usize) -> InventoryFuture<'_, ItemStack> {
        Box::pin(async move {
            let mut removed = ItemStack::EMPTY.clone();
            let mut guard = self.book.lock().await;
            std::mem::swap(&mut removed, &mut *guard);
            self.mark_dirty();
            removed
        })
    }

    fn remove_stack_specific(&self, _slot: usize, amount: u8) -> InventoryFuture<'_, ItemStack> {
        Box::pin(async move {
            let mut stack = self.book.lock().await;
            if stack.is_empty() {
                return ItemStack::EMPTY.clone();
            }
            let res = stack.split(amount);
            self.mark_dirty();
            res
        })
    }

    fn set_stack(&self, _slot: usize, stack: ItemStack) -> InventoryFuture<'_, ()> {
        Box::pin(async move {
            *self.book.lock().await = stack;
            self.mark_dirty();
        })
    }

    fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Relaxed);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Clearable for LecternBlockEntity {
    fn clear(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            *self.book.lock().await = ItemStack::EMPTY.clone();
            self.mark_dirty();
        })
    }
}

impl PropertyDelegate for LecternBlockEntity {
    fn get_property(&self, index: i32) -> i32 {
        let page=self.page;
        match index {
            0 => page,
            _ => 0
        }
    }

    fn set_property(&self, index: i32, value: i32) {}

    fn get_properties_size(&self) -> i32 {
        1
    }
}
