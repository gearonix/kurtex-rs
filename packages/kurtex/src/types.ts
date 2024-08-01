type Awaitable<T> = T | Promise<T>

export type TestCallback = () => Awaitable<void>
export type TestFactory = (
  cb: (name: string, fn: TestCallback) => void
) => Awaitable<void>

// TODO: improve namings

export interface TaskCell {
  (identifier: string, fn: TestCallback): void
}

export interface CreateNodeCell {
  (identifier: string): void
}

export interface Test extends TaskCell {
  skip: TaskCell
  only: TaskCell
  todo: TaskCell
}

export interface CreateNode extends CreateNodeCell {
  skip: CreateNodeCell
  only: CreateNodeCell
  todo: CreateNodeCell
}

export type CollectorRunMode = 'run' | 'skip' | 'only' | 'todo'

export interface KurtexInternals {
  registerCollectorTask: (
    identifier: string,
    callback: TestCallback,
    mode: CollectorRunMode
  ) => void
  registerCollectorNode: (
    identifier: string,
    factory: TestFactory,
    runMode: CollectorRunMode
  ) => void
}

export interface KurtexPublicApi {
  test: Test
  it: Test
  createNode: CreateNode
  suite: CreateNode
  describe: Test
}

export type ObjectEntry<T> = {
  [Key in Extract<keyof T, string>]: [Key, Exclude<T[Key], undefined>]
}[Extract<keyof T, string>]

declare global {
  // eslint-disable-next-line @typescript-eslint/no-namespace
  namespace Deno {
    interface DenoCore {
      ops: {
        op_register_collector_task: (
          identifier: string,
          callback: TestCallback,
          mode: CollectorRunMode
        ) => void
        // TODO: return type + identical types
        op_register_collector_node: (
          identifier: string,
          factory: TestFactory,
          mode: CollectorRunMode
        ) => unknown
      } & Record<string, (...args: any[]) => unknown>
    }

    export const core: DenoCore
  }

  const __kurtexInternals__: KurtexInternals

  const test: KurtexPublicApi['test']
  const it: KurtexPublicApi['it']
  const createNode: KurtexPublicApi['createNode']
  const suite: KurtexPublicApi['suite']
  const describe: KurtexPublicApi['describe']
}
