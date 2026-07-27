wit_bindgen::generate!({
    path: "main.wit"
});

export!(App);

struct App;

impl Guest for App {
    fn run() -> i32 {
        todo!();
    }
}
