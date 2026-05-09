use std::any::Any;
use std::pin::Pin;
use std::sync::Arc;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_data::screen::WindowType;
use pumpkin_world::block::entities::PropertyDelegate;
use crate::screen_handler::{InventoryPlayer, ItemStackFuture, ScreenHandler, ScreenHandlerBehaviour, ScreenProperty};

pub struct LecternScreenHandler {
    behaviour: ScreenHandlerBehaviour,
    property_delegate: Arc<dyn PropertyDelegate>, // for page (Integer) and Book (NBT)
}

impl LecternScreenHandler {
    pub async fn new(
        sync_id: u8,
        property_delegate: Arc<dyn PropertyDelegate>,
    ) -> Self{

        struct LecternScreenListener;
        impl crate::screen_handler::ScreenHandlerListener for LecternScreenListener {
            fn on_property_update<'a>(
                &'a self,
                screen_handler: &'a ScreenHandlerBehaviour,
                property: u8,
                value: i32,
            ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> {
                Box::pin(async move {
                    if let Some(sync_handler) = screen_handler.sync_handler.as_ref() {
                        sync_handler
                            .update_property(screen_handler, i32::from(property), value)
                            .await;
                    }
                })
            }
        }

        let mut handler = Self {
            behaviour: ScreenHandlerBehaviour::new(sync_id, Some(WindowType::BrewingStand)),
            property_delegate: property_delegate.clone(),
        };

        // Index 0: Book, 1: Page
        handler.add_property(ScreenProperty::new(property_delegate.clone(), 0));
        handler.add_property(ScreenProperty::new(property_delegate.clone(), 1));

        handler.add_listener(Arc::new(LecternScreenListener)).await;

        handler
    }

}

impl ScreenHandler for LecternScreenHandler {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn get_behaviour(&self) -> &ScreenHandlerBehaviour {
        &self.behaviour
    }

    fn get_behaviour_mut(&mut self) -> &mut ScreenHandlerBehaviour {
        &mut self.behaviour
    }

    fn quick_move<'a>(&'a mut self, _player: &'a dyn InventoryPlayer, _slot_index: i32) -> ItemStackFuture<'a> {
        Box::pin(async move { ItemStack::EMPTY.clone() })
    }
}

pub async fn create_lectern(sync_id: u8, property_delegate: Arc<dyn PropertyDelegate>) -> LecternScreenHandler {
    LecternScreenHandler::new(sync_id, property_delegate).await
}