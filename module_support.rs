fn module_support() -> bool {
    #[wasm_bindgen]
    pub struct Tester(Rc<Cell<bool>>);

    #[wasm_bindgen]
    impl Tester {
        #[wasm_bindgen(getter = type)]
        pub fn type_(&self) {
            self.0.set(true)
        }
    }

    let tester = Rc::new(Cell::new(false));
    let worker_options = WorkerOptions::from(JsValue::from(Tester(Rc::clone(&tester))));
    Worker::new_with_options("data:,", &worker_options).unwrap();

    tester.get()
}
