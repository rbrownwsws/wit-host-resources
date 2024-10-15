use crate::bindings::exports::example::addons::addon::Note;

mod bindings;

struct Component;

impl bindings::exports::example::addons::addon::Guest for Component {
    fn before_add_note(note: &Note) {
        println!("addon  (pre): {}", note.text());

        note.set_text("Hello Wasm!");

        println!("addon (post): {}", note.text());
    }
}

bindings::export!(Component with_types_in bindings);
