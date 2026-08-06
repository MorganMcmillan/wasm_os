# Blaze-64: Fast, Simple computers

Blaze-64 is a fantasy computer that runs on Webassembly and allows users to quickly write their own programs. It comes with an event system, graphics library, and other things you would typically find in an operating system.

## Embedding Blaze-64

Blaze-64 is designed to be embedded into your game as a mod. Players can use it to automate tasks or express their creativity.

Functionality is exposed to Blaze-64 through the `Driver` trait. Drivers expose functions inside `register_functions`. 

Example of a logger driver:

```rust
let name = self.name();

linker.func_wrap(name, "log_message", move |ctx: ProcessContext<T>, msg_ptr: i32, msg_len: u32| {
    let message = system_functions::get_memory(&ctx, msg_ptr, msg_len);

    ctx.data().kernel.get_driver::<Logger>(id).log(message);
})?;
```
```
```
