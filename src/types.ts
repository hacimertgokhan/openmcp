export type McpDefinition = {
  type?: 'local' | 'remote' | string
  command?: string[]
  url?: string
  enabled?: boolean
  environment?: Record<string, string>
  [key: string]: unknown
}

export type ConfigFile = {
  path: string
  label: string
  exists: boolean
  mcp_count: number
}

export type McpServer = {
  name: string
  source_path: string
  scope: string
  server_type: string
  enabled: boolean
  command?: string[]
  url?: string
  environment_keys: string[]
}

export type AppState = {
  configs: ConfigFile[]
  servers: McpServer[]
}

export type CatalogItem = {
  name: string
  title: string
  provider: string
  description: string
  tags: string[]
  definition: McpDefinition
  installNote?: string
}
