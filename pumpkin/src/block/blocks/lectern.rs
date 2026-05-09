use std::sync::Arc;
use tokio::sync::Mutex;
use crate::block::registry::BlockActionResult;
use crate::block::{
    BlockBehaviour, BlockFuture, BrokenArgs, NormalUseArgs, OnPlaceArgs, PlacedArgs,
    UseWithItemArgs,
};
use crate::entity::Entity;
use crate::entity::item::ItemEntity;
use crate::world::World;
use pumpkin_data::{translation, Block};
use pumpkin_data::block_properties::{BlockProperties, LecternLikeProperties};
use pumpkin_data::entity::EntityType;
use pumpkin_inventory::generic_container_screen_handler::create_hopper;
use pumpkin_inventory::lectern::lectern_screen_handler::create_lectern;
use pumpkin_inventory::player::player_inventory::PlayerInventory;
use pumpkin_inventory::screen_handler::{BoxFuture, InventoryPlayer, ScreenHandlerFactory, SharedScreenHandler};
use pumpkin_macros::pumpkin_block;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::text::TextComponent;
use pumpkin_world::block::entities::BlockEntity;
use pumpkin_world::BlockStateId;
use pumpkin_world::block::entities::lectern::LecternBlockEntity;
use pumpkin_world::inventory::Inventory;
use pumpkin_world::world::BlockFlags;

struct LecternScreenFactory(
    Arc<dyn pumpkin_world::block::entities::PropertyDelegate>,
);
impl ScreenHandlerFactory for LecternScreenFactory {
    fn create_screen_handler<'a>(&'a self, sync_id: u8, player_inventory: &'a Arc<PlayerInventory>, player: &'a dyn InventoryPlayer) -> BoxFuture<'a, Option<SharedScreenHandler>> {
        Box::pin(async move {
            let property_delegate=self.0.clone();
            let concrete_handler = create_lectern(sync_id, property_delegate).await;

            let concrete_arc = Arc::new(Mutex::new(concrete_handler));

            Some(concrete_arc as SharedScreenHandler)
        })
    }

    fn get_display_name(&self) -> TextComponent {
        TextComponent::translate_cross(
            translation::java::CONTAINER_LECTERN,
            translation::bedrock::TILE_LECTERN_NAME,
            &[],
        )
    }
}
#[pumpkin_block("minecraft:lectern")]
pub struct LecternBlock;

impl LecternBlock {
    async fn update_lectern_state(
        has_book: bool,
        block: &Block,
        position: &BlockPos,
        world: &Arc<World>,
        props: &mut LecternLikeProperties,
    ) {
        props.has_book = has_book;
        world
            .set_block_state(position, props.to_state_id(block), BlockFlags::NOTIFY_ALL)
            .await;
    }
}

impl BlockBehaviour for LecternBlock {
    fn placed<'a>(&'a self, args: PlacedArgs<'a>) -> BlockFuture<'a, ()> {
        Box::pin(async move {
            let block_entity = LecternBlockEntity::new(*args.position);
            args.world.add_block_entity(Arc::new(block_entity)).await;
        })
    }

    fn on_place<'a>(&'a self, args: OnPlaceArgs<'a>) -> BlockFuture<'a, BlockStateId> {
        Box::pin(async move {
            let mut props = LecternLikeProperties::default(args.block);
            props.facing = args
                .player
                .living_entity
                .entity
                .get_horizontal_facing()
                .opposite();
            props.to_state_id(args.block)
        })
    }

    fn normal_use<'a>(&'a self, args: NormalUseArgs<'a>) -> BlockFuture<'a, BlockActionResult> {
        Box::pin(async move {
            if let Some(block_entity) = args.world.get_block_entity(args.position).await
                && let Some(lectern_entity) =
                    block_entity.as_any().downcast_ref::<LecternBlockEntity>()
            {
                let book = lectern_entity.remove_stack(0).await;
                if !book.is_empty() {
                    // Logic to give the book to the player
                    // Need to find a proper way to give items to player. For now skip.

                    if let Some(pd)=block_entity.clone().to_property_delegate(){
                        println!("Opening Screen...");
                        args.player.open_handled_screen(&LecternScreenFactory(pd),Some(*args.position)).await;
                    }


                    let mut props = LecternLikeProperties::from_state_id(
                        args.world.get_block_state(args.position).await.id,
                        args.block,
                    );
                    Self::update_lectern_state(
                        false,
                        args.block,
                        args.position,
                        args.world,
                        &mut props,
                    )
                    .await;
                    return BlockActionResult::Success;
                }
            }
            BlockActionResult::Pass
        })
    }

    fn use_with_item<'a>(
        &'a self,
        args: UseWithItemArgs<'a>,
    ) -> BlockFuture<'a, BlockActionResult> {
        Box::pin(async move {
            let mut item_stack = args.item_stack.lock().await;

            // Check if it's a book
            if item_stack.item.registry_key.contains("book")
                && let Some(block_entity) = args.world.get_block_entity(args.position).await
                && let Some(lectern_entity) =
                    block_entity.as_any().downcast_ref::<LecternBlockEntity>()
                && lectern_entity.book.lock().await.is_empty()
            {
                let book = item_stack.split_unless_creative(args.player.gamemode.load(), 1);
                lectern_entity.set_stack(0, book).await;

                let mut props = LecternLikeProperties::from_state_id(
                    args.world.get_block_state(args.position).await.id,
                    args.block,
                );
                Self::update_lectern_state(true, args.block, args.position, args.world, &mut props)
                    .await;
                return BlockActionResult::Success;
            }
            BlockActionResult::Pass
        })
    }

    fn broken<'a>(&'a self, args: BrokenArgs<'a>) -> BlockFuture<'a, ()> {
        Box::pin(async move {
            if let Some(block_entity) = args.world.get_block_entity(args.position).await
                && let Some(lectern_entity) =
                    block_entity.as_any().downcast_ref::<LecternBlockEntity>()
            {
                let book = lectern_entity.remove_stack(0).await;
                if !book.is_empty() {
                    // Drop the book item
                    let entity = Entity::new(
                        args.world.clone(),
                        Vector3::new(
                            f64::from(args.position.0.x) + 0.5,
                            f64::from(args.position.0.y) + 0.5,
                            f64::from(args.position.0.z) + 0.5,
                        ),
                        &EntityType::ITEM,
                    );
                    let item_entity = ItemEntity::new(entity, book).await;
                    args.world.spawn_entity(Arc::new(item_entity)).await;
                }
            }
        })
    }
}

