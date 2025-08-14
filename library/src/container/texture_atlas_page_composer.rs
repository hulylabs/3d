use crate::material::atlas_region_mapping::AtlasRegionMappingBuilder;
use crate::material::material_properties::MaterialProperties;
use crate::material::texture_atlas_regions_warehouse::TextureAtlasRegionsWarehouse;
use crate::material::texture_reference::TextureReference;
use crate::material::texture_region::TextureRegion;
use crate::utils::bitmap_utils::{write_sub_bitmap, BitmapSize, ImmutableBitmapReference, MutableBitmapReference};
use crate::utils::version::Version;
use cast::i32;
use cgmath::Vector2;
use etagere::{AllocId, AtlasAllocator, Size};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type AtlasRegionUid = AllocId;

pub struct TextureAtlasPageComposer {
    atlas_page_buffer: Vec<u8>,
    allocator: AtlasAllocator,
    allocations: HashMap<AllocId, TextureRegion>,
    warehouse: Rc<RefCell<TextureAtlasRegionsWarehouse>>,
    page_size: BitmapSize,
    atlas_page_data_version: Version,
}

impl TextureAtlasPageComposer {
    const DEFENSIVE_BORDER_SIZE: usize = 1;

    #[must_use]
    pub(crate) fn new(page_size: BitmapSize, warehouse: Rc<RefCell<TextureAtlasRegionsWarehouse>>) -> Self {
        Self {
            atlas_page_buffer: vec![0; page_size.bytes_in_bitmap()],
            allocator: AtlasAllocator::new(Size::new(page_size.width() as i32, page_size.height() as i32)),
            allocations: HashMap::new(),
            warehouse,
            page_size,
            atlas_page_data_version: Version(0),
        }
    }



    #[must_use]
    pub fn allocate(&mut self, bitmap: ImmutableBitmapReference) -> Option<AtlasRegionUid> {
        const BORDER: usize = TextureAtlasPageComposer::DEFENSIVE_BORDER_SIZE;
        let width = i32(bitmap.size().width() + BORDER * 2).ok()?;
        let height = i32(bitmap.size().height() + BORDER * 2).ok()?;
        let allocation = self.allocator.allocate(Size::new(width, height))?;

        let allocated_rectangle = allocation.rectangle;
        let page_width = self.page_size.width() as f32;
        let page_height = self.page_size.height() as f32;
        let pixel_x = allocated_rectangle.min.x as usize + BORDER;
        let pixel_y = allocated_rectangle.min.y as usize + BORDER;
        let u = pixel_x as f32 / page_width;
        let v = pixel_y as f32 / page_height;
        let width = bitmap.size().width() as f32 / page_width;
        let height = bitmap.size().height() as f32 / page_height;
        let region = TextureRegion::new(Vector2::new(u, v), Vector2::new(width, height));
        self.allocations.insert(allocation.id, region);

        write_sub_bitmap(MutableBitmapReference::new(&mut self.atlas_page_buffer, self.page_size), bitmap, pixel_x, pixel_y);

        self.atlas_page_data_version += 1;

        Some(allocation.id)
    }

    pub fn map_into(&mut self, region: AtlasRegionUid, mapping: AtlasRegionMappingBuilder, target: &mut MaterialProperties) -> anyhow::Result<()> {
        let allocation = self.allocations.get(&region)
            .ok_or_else(|| anyhow::anyhow!("allocation failed"))?;

        let atlas_region_mapping = mapping.build(allocation.clone());
        let mapped_region_uid = self.warehouse.borrow_mut().add_region(atlas_region_mapping);

        let bitmap_index = self.warehouse.borrow_mut().get_region_index(mapped_region_uid)
            .ok_or_else(|| anyhow::anyhow!("region index not found"))?;

        target.set_albedo_texture(TextureReference::Bitmap(bitmap_index));
        Ok(())
    }
    
    pub(crate) fn try_commit<ConsumerDelegate: FnOnce(Version, &[u8])>(&self, consumer_data_version_or_none: Option<Version>, consume: ConsumerDelegate) {
        if  consumer_data_version_or_none != Some(self.atlas_page_data_version) {
            consume(self.atlas_page_data_version, &self.atlas_page_buffer);
        }
    }

    #[must_use]
    pub fn page_size(&self) -> BitmapSize {
        self.page_size
    }
}