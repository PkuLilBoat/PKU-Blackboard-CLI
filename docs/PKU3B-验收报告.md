# PKU3B 验收报告

## 1. 本轮目标

本轮做了四件事：

- 对主要功能逐项实测
- 发现问题后直接优化
- 把结构化输出从 JSON 切换到 Markdown
- 整理文档并准备推送 GitHub

## 2. 这轮实际优化了什么

### 2.1 结构化输出改成 Markdown

现在原来走 `--json` 的主要命令，主输出模式改成了 `--markdown`：

- `course --markdown`
- `cache --markdown`
- `announcement --markdown`
- `document --markdown`
- `coursetable --markdown`
- `assignment --markdown`
- `find --markdown`
- `search --markdown`
- `tree --markdown`
- `video --markdown`

同时为了兼容旧调用，保留了 `--json` 作为别名；但实际输出已经是 Markdown。

### 2.2 Markdown 渲染器新增

新增了 `src/cli/markdown_output.rs`，用于把结构化结果稳定渲染成：

- 标题 + 元信息
- 扁平列表表格
- 嵌套对象分节
- 下载/动作结果摘要

这样做的目标是：

- 人直接看更舒服
- AI 直接读也足够稳定
- 不再要求所有调用方先做 JSON 解析

### 2.3 命令帮助与说明文档同步

这轮还同步更新了：

- `README.md`
- `docs/README.md`
- `docs/PKU3B-AI-CLI-通俗说明.md`
- `docs/PKU3B-功能总览.md`
- `docs/PKU3B-CLI-help-full.txt`

## 3. 这轮实测过的功能

### 3.1 已直接实跑的主要读操作

- `cache show`
- `course list`
- `course entries`
- `announcement list`
- `announcement show`
- `document list`
- `document show`
- `find`
- `search`
- `tree list`
- `tree find`
- `tree kinds`
- `video list`
- `coursetable`
- `coursetable --raw`
- `config`

### 3.2 已直接实跑的低风险写操作

- `assignment download`
- `document download`
- `video download`
- `cache clean`

### 3.3 已做安全校验但未执行真实破坏性/远程变更的命令

这类命令为了避免影响真实教学网状态，没有对线上做真实写入，只验证了帮助、入口或报错路径：

- `assignment submit`
- `syllabus set`
- `syllabus unset`
- `syllabus launch`
- `init`（只看帮助，没有覆盖现有配置）

## 4. 本轮观察到的结果

### 4.1 正常通过

- 主要资源读取都能返回可读 Markdown
- 下载类动作都能正常完成
- `video download` 仍能产出 MP4
- `document download` 和 `assignment download` 能正常落地
- `coursetable` 在 portal 受限时仍能 fallback

### 4.2 发现的现实限制

- `syllabus show` 本轮返回上游 IAAA `E21`：尝试次数过多，需要半小时后再试
- 在本轮最后一次集中回归时，Blackboard 读路径也因为短时间内频繁登录而触发了同样的
  IAAA `E21` 限流；这说明当前主要限制已经不在 CLI 本地逻辑，而在上游登录频控
- `ttshitu test` 返回未配置，这是预期行为
- `bark test` 返回未配置，这是预期行为
- `thesis-lib` 的帮助路径正常，但本轮没有把真实检索结果纳入通过项

## 5. 当前结论

当前版本已经可以认为：

- 主体命令面可用
- Markdown 输出已切换完成
- 本地文档已经补齐到可以直接对外介绍项目

但不应理解成“永远 100% 不会再遇到问题”。

更准确的结论是：

- 当前项目里程碑可交付
- 核心功能可运行
- 外部依赖类问题仍可能随着上游系统状态变化出现

## 6. 推荐阅读顺序

- `README.md`
- `docs/PKU3B-AI-CLI-通俗说明.md`
- `docs/PKU3B-功能总览.md`
- `docs/PKU3B-CLI-help-full.txt`
