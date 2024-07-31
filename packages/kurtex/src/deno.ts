import type {
  CollectorRunMode,
  KurtexInternals,
  KurtexPublicApi,
  ObjectEntry,
  TaskCell,
  Test,
  TestCallback
} from '@/types'

const { core } = Deno
const { ops } = core

function registerTaskImpl(runMode: CollectorRunMode): TaskCell {
  return (identifier: string, callback: TestCallback) => {
    kurtexInternalGate.registerCollectorTask(
      identifier,
      runMode === 'todo' ? () => {} : callback,
      runMode
    )
  }
}

const registerTask = registerTaskImpl('run') as Test
registerTask.only = registerTaskImpl('only')
registerTask.skip = registerTaskImpl('skip')
registerTask.todo = registerTaskImpl('todo')

const kurtexInternalGate = {
  registerCollectorTask(identifier, callback, mode = 'run') {
    ops.op_register_collector_task(identifier, callback, mode)
  }
} satisfies KurtexInternals

export const kurtexPublicApi = {
  test: registerTask
} satisfies KurtexPublicApi

function registerApiGlobally() {
  const publicApi = Object.entries(
    kurtexPublicApi
  ) as ObjectEntry<KurtexPublicApi>[]

  publicApi.forEach(([key, value]) => {
    // @ts-expect-error globalThis is not supported by Deno plugin
    globalThis[key] = value
  })
}

// @ts-expect-error globalThis is not supported by Deno plugin
globalThis._kurtexInternals = kurtexInternalGate

registerApiGlobally()
