# PKU3B 功能总览

这份文档是当前 `pku3b` 项目的功能地图，面向两类人：

- 直接在终端里使用的用户
- 想把 `pku3b` 当成 AI / 脚本后端来调用的人

如果只看一句话，可以这样理解：

`pku3b` 是一个面向北大教学网的单文件 Rust CLI，它既能给人直接用，也能给 AI 和自动化流程稳定调用。

## 1. 项目定位

这个项目的核心不是“做一个聊天助手”，而是“做一个可靠的低层能力工具箱”。

它主要负责三类事情：

- 读取课程、公告、文档、树节点、视频、作业、课表等教学网资源
- 执行下载、提交、缓存处理、配置初始化等原子操作
- 用稳定的终端输出和 JSON 输出，把结果交给人或上层 AI

这意味着：

- 它适合做后端能力层
- 它不负责高层决策，比如“本周最重要的任务是什么”
- 它强调单命令、单职责、可组合

## 2. 当前顶层命令

当前顶层命令包括：

- `assignment`：作业列表、下载、提交
- `coursetable`：课表读取
- `course`：课程列表与课程入口
- `announcement`：课程公告列表与详情
- `document`：课程文档列表、详情、下载
- `find`：按标题做确定性查找
- `search`：跨课程结构化搜索
- `tree`：课程内容树列表、查找、类型统计
- `video`：课程回放列表与下载
- `syllabus`：选课相关操作
- `ttshitu`：图形验证码识别
- `bark`：Bark 通知配置
- `thesis-lib`：学位论文检索
- `init`：初始化账号密码
- `config`：查看和修改配置
- `cache`：查看缓存和清理缓存

## 3. AI / 自动化最有价值的命令面

如果从“适合 AI 调用”的角度看，当前最重要的是下面这些能力已经支持稳定 JSON：

- `course --json`
- `cache --json`
- `announcement --json`
- `document --json`
- `coursetable --json`
- `assignment --json`
- `find --json`
- `search --json`
- `tree --json`
- `video --json`

这批命令适合做：

- AI agent 的底层检索能力
- shell 脚本自动化
- GUI/TUI/Web 前端的数据后端
- 本地知识整理、课程资料下载、批处理

## 4. 详细功能介绍

### 4.1 `assignment`

用途：

- 列出作业
- 下载作业附件
- 提交作业

典型子命令：

- `pku3b assignment list`
- `pku3b assignment download <ID>`
- `pku3b assignment submit <ID> <PATH>`

适合场景：

- 看当前课程还有哪些作业
- 批量保存作业要求
- 自动化提交作业文件
- 给 AI 提供结构化作业列表

### 4.2 `coursetable`

用途：

- 读取个人课表
- 输出结构化课表数据

特点：

- 支持 JSON 输出
- 在某些 portal 路径 OTP 受限时，当前实现会走 Blackboard calendar fallback

适合场景：

- 做个人课表同步
- 做课程时间可视化
- 给 AI 提供课程安排上下文

### 4.3 `course`

用途：

- 获取课程列表
- 获取课程菜单入口

典型子命令：

- `pku3b course list`
- `pku3b course entries`

适合场景：

- 看当前学期有哪些课
- 看所有可见学期课程
- 给后续 document / announcement / tree 等能力提供课程上下文

### 4.4 `announcement`

用途：

- 获取课程公告列表
- 按 ID 获取公告详情

典型子命令：

- `pku3b announcement list`
- `pku3b announcement show <ID>`

适合场景：

- 拉取某门课或全局公告
- 做公告归档
- 给 AI 提供课程通知内容

### 4.5 `document`

用途：

- 获取课程文档列表
- 查看文档详情
- 下载文档附件或正文负载

典型子命令：

- `pku3b document list`
- `pku3b document show <ID>`
- `pku3b document download <ID>`

适合场景：

- 批量归档课程材料
- 按课程整理资料
- 抽取文档标题和附件信息
- 给 AI 建立课程资料索引

### 4.6 `find`

用途：

- 按标题进行确定性查找

特点：

- 会做规范化匹配
- 更适合“我大概知道标题，只想稳定找到它”

适合场景：

- 找某个 week 文档
- 找某个公告标题
- 把人工输入变成稳定资源定位

### 4.7 `search`

用途：

- 跨课程搜索结构化内容

特点：

- 比 `find` 更像全局搜索
- 适合模糊定位内容

适合场景：

- 搜多个课程里的同类资料
- 搜 week / topic / keyword
- 让 AI 做跨课程资料召回

### 4.8 `tree`

用途：

- 获取课程树
- 查找树节点
- 按类型统计树节点

典型子命令：

- `pku3b tree list`
- `pku3b tree find <COURSE_ID> <QUERY>`
- `pku3b tree kinds`

适合场景：

- 看课程内容组织结构
- 找 week / folder / learning module
- 给 AI 提供课程导航树

### 4.9 `video`

用途：

- 获取课程回放列表
- 下载课程回放

典型子命令：

- `pku3b video list`
- `pku3b video download <ID>`

特点：

- 下载产物是 MP4
- 依赖 `ffmpeg`
- 已验证真实下载流程可跑通

适合场景：

- 批量保存课程录像
- 离线整理学习资料
- 让 AI 先检索列表再下载指定视频

### 4.10 `syllabus`

用途：

- 选课相关操作

适合场景：

- 快捷选课配置
- 自动循环选课
- 跟其他通知能力联动

### 4.11 `ttshitu`

用途：

- 图形验证码识别配置与测试

适合场景：

- 某些选课/登录流程需要验证码时做辅助

### 4.12 `bark`

用途：

- Bark 推送通知初始化和测试

适合场景：

- 选课通知
- 任务完成提醒
- 自动化流程触发后的移动端提醒

### 4.13 `thesis-lib`

用途：

- 学位论文检索

适合场景：

- 本地论文查找
- 辅助学术资料检索

### 4.14 `init`

用途：

- 初始化教学网账号和密码

适合场景：

- 首次配置
- 重置登录配置

### 4.15 `config`

用途：

- 查看和修改本地配置

适合场景：

- 检查当前配置
- 修改通知、缓存、账号等设置

### 4.16 `cache`

用途：

- 查看缓存占用
- 清理缓存

适合场景：

- 清理本地空间
- 检查缓存是否过大
- 自动化维护本地状态

## 5. 项目当前的优势

当前这个项目已经具备几个很明显的优势：

- 单文件 Rust CLI，部署轻
- 既保留了原来对人友好的终端用法，也在补 AI 友好的 JSON 合同
- 支持真实的下载和提交动作，不只是只读查询
- 可以作为上层 AI skill、agent、脚本系统的稳定后端

## 6. 当前边界与现实情况

这个项目现在已经完成了当前定义的里程碑，但不应该理解成“永远不会再遇到任何问题”。

更准确的说法是：

- 关键命令面已经打通
- 核心 JSON 能力已经落地
- 单测和关键真人 smoke test 已经过关
- 但更深的集成测试、更多边缘路径和未来扩展仍然可以继续加强

特别是课表这类依赖 portal 的路径，现实中仍可能受到 OTP 或上游页面变化影响，因此当前实现保留了 fallback 方案。

## 7. 文档入口建议

建议按这个顺序阅读：

- `docs/PKU3B-AI-CLI-通俗说明.md`
- `docs/PKU3B-功能总览.md`
- `docs/PKU3B-AI-CLI-spec.md`
- `docs/PKU3B-AI-CLI-testing.md`
- `docs/PKU3B-AI-CLI-completion-audit.md`

## 8. 原始 help 导出

如果你想看完整原始 help 文本，可以直接看：

- `docs/PKU3B-CLI-help-full.txt`
