import type {
  CollectorRunMode,
  CreateNode,
  KurtexInternals,
  KurtexPublicApi,
  ObjectEntry,
  TaskCell,
  Test,
  TestCallback,
  TestFactory
} from '@/types'

const { core } = Deno
const { ops } = core

function registerTaskImpl(runMode: CollectorRunMode): TaskCell {
  return (identifier: string, callback: TestCallback | undefined) => {
    kurtexInternalGate.registerCollectorTask(
      identifier,
      callback && runMode !== 'todo' ? callback : () => {},
      runMode
    )
  }
}

function registerNodeImpl(runMode: CollectorRunMode) {
  return (identifier: string, factory?: TestFactory) => {
    kurtexInternalGate.registerCollectorNode(
      identifier,
      factory && runMode !== 'todo' ? factory : () => {},
      runMode
    )
  }
}

const registerTask = registerTaskImpl('run') as Test
registerTask.only = registerTaskImpl('only')
registerTask.skip = registerTaskImpl('skip')
registerTask.todo = registerTaskImpl('todo')

const registerNode = registerNodeImpl('run') as CreateNode

registerNode.only = registerNodeImpl('only')
registerNode.skip = registerNodeImpl('skip')
registerNode.todo = registerNodeImpl('todo')

const kurtexInternalGate = {
  registerCollectorTask(identifier, callback, mode) {
    ops.op_register_collector_task(identifier, callback, mode)
  },
  registerCollectorNode(identifier, factory, mode) {
    ops.op_register_collector_node(identifier, factory, mode)
  }
} satisfies KurtexInternals

const kurtexPublicApi = {
  test: registerTask,
  it: registerTask,
  createNode: registerNode,
  describe: registerNode,
  suite: registerNode
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
