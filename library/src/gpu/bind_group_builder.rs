use std::collections::HashMap;
use std::rc::Rc;
use wgpu::{BindGroup, Buffer};

pub(super) struct BindGroupBuilder<'a> {
    index: u32,
    label: Option<&'a str>,
    layout: wgpu::BindGroupLayout,
    accumulated_entries: HashMap<u32, Rc<Buffer>>,
}

impl<'a> BindGroupBuilder<'a> {
    #[must_use]
    pub(super) fn new(index: u32, label: Option<&'a str>, layout: wgpu::BindGroupLayout) -> Self {
        Self { index, label, layout, accumulated_entries: HashMap::new() }
    }

    pub(super) fn add_entry(&mut self, slot: u32, buffer: Rc<Buffer>) -> &mut Self {
        self.accumulated_entries.insert(slot, buffer);
        self
    }

    #[must_use]
    pub(super) fn make_bind_group(&self, device: &wgpu::Device) -> BindGroup {
        let entries = {
            let mut entries = Vec::new();
            self.accumulated_entries.iter().for_each(
                |(slot_number, buffer)| {
                    entries.push(wgpu::BindGroupEntry {
                        binding: *slot_number,
                        resource: buffer.as_entire_binding(),
                    });
                });
            entries
        };
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout: &self.layout,
            entries: entries.as_slice(),
        })
    }

    #[must_use]
    pub fn index(&self) -> u32 {
        self.index
    }
}
