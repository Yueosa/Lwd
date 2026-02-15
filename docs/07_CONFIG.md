# 配置系统

## 配置层次

```
编译时嵌入 (include_str!)          运行时读写
─────────────────────────          ────────────
src/assets/blocks.json             generation.runtime.json
src/assets/biome.json              └ layers (层级覆盖)
src/assets/world.json              └ ui (UI 状态)
```

## 编译时配置

三个 JSON 文件通过 `include_str!()` 嵌入二进制，启动时一次性解析。

### blocks.json（43 种方块）

```json
{
  "0": { "name": "空气", "rgba": [135, 206, 235, 255], "description": "...", "category": "基础" },
  "1": { "name": "泥土", "rgba": [139, 90, 43, 255], "description": "...", "category": "基础" },
  ...
}
```

- **Key**: 方块 ID（u8，0-255）
- **rgba**: 画布渲染颜色
- **category**: 分组标签（基础 / 植物 / 矿石 / 液体 / 装饰 等 13 类）

### biome.json（6 种环境）

```json
{
  "0": { "key": "ocean", "name": "海洋", "overlay_color": [0, 105, 148, 128], "generation_weight": 1.0, "description": "..." },
  "1": { "key": "forest", "name": "森林", "overlay_color": [34, 139, 34, 128], ... },
  ...
}
```

- **Key**: Biome ID（u8）
- **overlay_color**: 覆盖层半透明颜色 RGBA
- **generation_weight**: 生成权重
- **key**: 程序内标识符

### world.json

```json
{
  "world_sizes": {
    "small":  { "width": 4200, "height": 1200, "description": "小世界" },
    "medium": { "width": 6400, "height": 1800, "description": "..." },
    "large":  { "width": 8400, "height": 2400, "description": "..." },
    "custom": { "width": null, "height": null, "description": "自定义" }
  },
  "layers": {
    "space":       { "start_percent": 0,  "end_percent": 5,   "description": "太空层" },
    "surface":     { "start_percent": 5,  "end_percent": 25,  "description": "地表层" },
    "underground": { "start_percent": 25, "end_percent": 35,  "description": "地下层" },
    "cavern":      { "start_percent": 35, "end_percent": 80,  "description": "洞穴层" },
    "hell":        { "start_percent": 80, "end_percent": 100, "description": "地狱层" }
  }
}
```

- **world_sizes**: 预设世界尺寸（`null` 表示自定义待实现）
- **layers**: 默认层级垂直分布百分比

## 运行时配置

### generation.runtime.json

运行时读写的持久化文件，保存 UI 状态和层级覆盖。

```json
{
  "layers": {
    "space":       { "start_percent": 0,  "end_percent": 5 },
    "surface":     { "start_percent": 5,  "end_percent": 25 },
    "underground": { "start_percent": 25, "end_percent": 35 },
    "cavern":      { "start_percent": 35, "end_percent": 80 },
    "hell":        { "start_percent": 80, "end_percent": 100 }
  },
  "ui": {
    "world_size": "small",
    "show_biome_overlay": true,
    "show_layer_overlay": true
  }
}
```

### 路径解析

使用 `runtime_json_path()` 函数：
1. 优先取 `std::env::current_exe().parent()`
2. 找不到 exe 路径时回退到 `std::env::current_dir()`

### 读写机制

- 读：`app.rs::new()` 启动时加载 → 恢复 UI 状态 + 层级配置
- 写：`merge_runtime_field(key, value)` — 通用增量合并工具
  - 读取整个 JSON → 修改指定字段 → 写回
  - 被 `layer_config.rs` 和 `app.rs` 共用

## 加载流程

```
程序启动
  ├─ include_str!("blocks.json") → BTreeMap<u8, BlockConfig>
  │    └─ build_block_definitions() → Vec<BlockDefinition>
  │    └─ build_color_lut() → [Color32; 256]
  ├─ include_str!("biome.json")  → BTreeMap<u8, BiomeConfig>
  │    └─ build_biome_definitions() → Vec<BiomeDefinition>
  ├─ include_str!("world.json")  → WorldConfig
  │    └─ WorldProfile::from_config()
  └─ runtime.json (可选)
       └─ 覆盖层级配置 + 恢复 UI 状态
```
