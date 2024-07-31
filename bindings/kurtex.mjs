const { core } = Deno
const { ops } = core

// TODO: rewrite to ts



const KurtexInternalSdk = {
    test: (identifier, callback) => {
        ops.op_register_task_t1rigger(identifier, callback, "run");
    }
}


function registerApiGlobally() {
    Object.entries(KurtexInternalSdk).forEach(([key, value]) => {
        globalThis[key] = value
    })
}


globalThis.__kurtex_internals__ = KurtexInternalSdk

registerApiGlobally()
