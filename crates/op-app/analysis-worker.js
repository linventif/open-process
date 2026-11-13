// The same Wasm build runs without starting eframe in a worker global scope.
self.onmessage = async ({data}) => {
    try {
        const bindings = await import(data.glue);
        await bindings.default({module_or_path: data.wasm});
        self.postMessage({ok: bindings.run_background_job(data.input)});
    } catch (error) {
        self.postMessage({error: String(error?.message || error)});
    }
};
