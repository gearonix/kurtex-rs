export type TestCallback = () => void | Promise<void>

export interface TaskCell {
  (identifier: string, fn: TestCallback): void
}

export interface Test extends TaskCell {
  skip: TaskCell
  only: TaskCell
  todo: TaskCell
}

export type CollectorRunMode = 'run' | 'skip' | 'only' | 'todo'

export interface KurtexInternals {
  registerCollectorTask: (
    identifier: string,
    callback: TestCallback,
    mode?: CollectorRunMode
  ) => void
}

export interface KurtexPublicApi {
  test: Test
}

export type ObjectEntry<T> = {
  [Key in Extract<keyof T, string>]: [Key, Exclude<T[Key], undefined>]
}[Extract<keyof T, string>]
