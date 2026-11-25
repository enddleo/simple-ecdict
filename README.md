# simple ecdict — 离线英汉词典（Windows 图形界面）

输入一个英文单词，立即返回：

- **音标**（IPA，自动规范化）
- **按词性分组的中文释义**（n. / adj. / adv. …）
- **词态变化**（复数、过去式、过去分词、现在分词、第三人称单数、比较级、最高级）

内置约 **42000 个有词频的常用词**，数据已直接编译进可执行文件，**完全离线、单文件分发**。

---

## 功能特性

| 特性 | 说明 |
|------|------|
| 离线查询 | 词库直接打包进二进制，无需联网、无需数据文件 |
| 词性分组 | 中文释义按词性自动分组显示（n. / vt. / vi. / adj. / adv. …） |
| 词态变化 | 复数、过去式、过去分词、现在分词、三单、比较级、最高级 |
| 大小写不敏感 | `China` / `china` / `CHINA` 均可命中 |
| 拼写建议 | 查不到时给出相近词建议（编辑距离 + 前缀匹配），可点击 |
| 回车搜索 | 输入后回车查询，查询后自动全选已输入内容并保持聚焦 |
| 点击全选 | 鼠标点击输入框时自动全选已有内容，便于连续查询 |
| 剪贴板识别 | 复制（Ctrl+C）英文单词后自动查询，默认启用 |
| 始终置顶 | 窗口常驻最前，便于对照原文 |
| 自适应高度 | 窗口根据结果行数自动调整高度（无需滚动），无结果时缩到最小 |

---

## 使用方法

双击 `ecdict-gui.exe` 运行。

- **输入框**：启动即聚焦，输入单词回车查询；查询后自动全选，直接输入新词即可连续查询。
- **剪贴板识别**：在任意应用中复制（Ctrl+C）一个英文单词，词典会自动查询该词（无需任何开关，默认启用）。
- **日志**：默认不写日志；如需调试，在命令行运行 `ecdict-gui.exe --log`，会在 exe 同目录生成 `ecdict.log`。

---

## 构建

### 依赖

- Rust 工具链（stable）
- Windows 需要 MSVC 或 MinGW-w64 链接器

### 构建

```powershell
cargo build --release
# 产物: target\release\ecdict-gui.exe（约 20 MB，运行时加载系统中文字体）
```

---

## 词库生成 / 更新

词库由 `tools/build_data.py` 从 ECDICT 数据生成 `src/word_data.rs`（静态嵌入）。

```bash
python tools/build_data.py <stardict.db> <src/word_data.rs> 42231
```

默认取全部有词频（frq>0）的 42231 个常用词（字母词、含中文释义、去重）。

---

## 数据来源与许可

- 词库数据：**[ECDICT](https://github.com/skywind3000/ECDICT)**（简明英汉字典增强版），MIT License。
- 本项目代码：MIT License。

> 说明：ECDICT 的词态变化（exchange）字段由算法自动生成，个别词条可能存在不完美之处（例如部分动词也生成了"复数"），属数据源本身特性，非本项目缺陷。

详见 [LICENSE](LICENSE) 与 [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)。

---

## 目录结构

```
ecdict/
├── Cargo.toml
├── Cargo.lock
├── LICENSE                  # MIT 许可证（本项目代码）
├── README.md
├── THIRD_PARTY_NOTICES.md   # 第三方数据（ECDICT）许可
├── src/
│   ├── lib.rs            # 词典核心库（索引/查询/解析/建议）
│   ├── word_data.rs      # 自动生成的词库数据（42231 词）
│   └── bin/
│       ├── ecdict-gui.rs # GUI 入口
│       └── selection.rs  # 剪贴板监控（复制即查）
└── tools/
    └── build_data.py     # 词库生成脚本
```
