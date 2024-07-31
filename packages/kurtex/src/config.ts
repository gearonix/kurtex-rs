export interface KurtexOptions {
  includes?: string[]
  excludes?: string[]
  parallel?: boolean
  watch?: boolean
}

export function defineConfig(config: KurtexOptions): KurtexOptions {
  return config
}
