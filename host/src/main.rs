use std::error::Error;
use wasmtime::component::{Component, Linker, Resource, ResourceTable};
use wasmtime::{Config, Engine, Store};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder, WasiView};

mod bindings {
    wasmtime::component::bindgen!({
        path: "../wit",
        world: "addons",

        with: {
            "example:addons/types/note": crate::Note,
        }
    });
}

use crate::bindings::Addons;

#[derive(Clone, Debug)]
pub struct Note {
    text: String,
}

impl Note {
    pub fn new() -> Self {
        Note {
            text: "EMPTY_NOTE".to_owned(),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn set_text(&mut self, text: &str) {
        // Do something to make having a setter make sense
        self.text = format!("NOTE: {}", text)
    }
}

struct HostState {
    resource_table: ResourceTable,
    wasi_ctx: WasiCtx,
}

impl WasiView for HostState {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }

    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl bindings::example::addons::types::HostNote for HostState {
    fn text(&mut self, self_: Resource<Note>) -> String {
        self.resource_table.get(&self_).unwrap().text().to_owned()
    }

    fn set_text(&mut self, self_: Resource<Note>, new_text: String) -> () {
        self.resource_table
            .get_mut(&self_)
            .unwrap()
            .set_text(&new_text);
    }

    fn drop(&mut self, _rep: Resource<Note>) -> wasmtime::Result<()> {
        // I assume this is never called because the host always owns the Note
        Ok(())
    }
}

impl bindings::example::addons::types::Host for HostState {}

struct AddonHost {
    #[allow(unused)]
    engine: Engine,
    store: Store<HostState>,
    instance: Addons,
}

impl AddonHost {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let mut config = Config::default();
        config.wasm_component_model(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker_sync(&mut linker)?;
        Addons::add_to_linker(&mut linker, |state| state)?;

        let mut builder = WasiCtxBuilder::new();
        builder.inherit_stdout();
        builder.inherit_stderr();

        let state = HostState {
            resource_table: ResourceTable::new(),
            wasi_ctx: builder.build(),
        };

        let mut store = Store::new(&engine, state);

        let component_bytes =
            std::fs::read("../guest/target/wasm32-wasip1/release/guest.wasm").unwrap();
        let component = Component::new(&engine, component_bytes)?;
        let instance = Addons::instantiate(&mut store, &component, &linker)?;

        Ok(AddonHost {
            engine,
            store,
            instance,
        })
    }

    // N.B. You are not allowed to change the signature of this function.
    pub fn before_add_note(&mut self, note: &mut Note) {
        let note_resource = self
            .store
            .data_mut()
            .resource_table
            .push(note.clone())
            .unwrap();

        self.instance
            .example_addons_addon()
            .call_before_add_note(&mut self.store, note_resource)
            .unwrap();

        // FIXME: This does not work because `note_resource` has been moved!
        // *note = self
        //     .store
        //     .data_mut()
        //     .resource_table
        //     .delete(note_resource)
        //     .unwrap();
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut addon_host = AddonHost::new()?;

    let mut note = Note::new();
    note.set_text("Hello World");

    println!("host   (pre): {}", note.text());

    addon_host.before_add_note(&mut note);

    println!("host  (post): {}", note.text());

    Ok(())
}
