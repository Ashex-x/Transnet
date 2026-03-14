# Transnet Frontend

🚀 **AI 驱动的智能翻译平台前端**

基于 Deep Space Aurora 设计语言的科技感翻译网站，具有玻璃拟态、霓虹灯效果和粒子动画。

## ✨ 特性

- 🎨 **Deep Space Aurora 设计** - 深空极光主题，电光青 + 等离子紫配色
- 💎 **玻璃拟态效果** - 毛玻璃卡片、发光边框、悬浮动画
- 🌌 **动态粒子背景** - Canvas 实现的粒子网络，鼠标交互
- ⚡ **流畅动画** - 基于物理的动画，弹簧效果
- 🌐 **多语言翻译** - 支持 6 种语言互译
- 🔐 **用户系统** - JWT 认证、注册、登录、个人中心
- 📜 **翻译历史** - 自动保存、筛选、分页
- ⭐ **收藏功能** - 收藏翻译、添加备注

## 🛠️ 技术栈

- **TypeScript** - 类型安全的 JavaScript
- **SCSS** - CSS 预处理器，模块化样式
- **Canvas API** - 粒子动画背景
- **Fetch API** - HTTP 请求
- **LocalStorage** - JWT Token 存储

## 📁 文件结构

```
static/
├── index.html              # HTML 入口
├── src/
│   ├── styles/
│   │   └── transnet.scss   # 主样式文件（设计系统）
│   ├── services/
│   │   ├── ApiService.ts   # API 接口封装
│   │   └── AuthService.ts  # 认证管理
│   ├── components/
│   │   ├── ParticleBackground.ts  # 粒子背景
│   │   ├── Translator.ts          # 翻译组件
│   │   ├── Header.ts              # 导航头部
│   │   └── Toast.ts               # 消息提示
│   ├── pages/
│   │   ├── HomePage.ts     # 首页/翻译页
│   │   ├── AuthPage.ts     # 登录/注册页
│   │   ├── HistoryPage.ts  # 历史记录页
│   │   ├── FavoritesPage.ts# 收藏页
│   │   └── ProfilePage.ts  # 个人中心
│   ├── app.ts              # 应用入口
│   └── router.ts           # 路由管理
└── dist/                   # 编译输出（构建时生成）
```

## 🚀 快速开始

### 1. 安装依赖

```bash
cd static
npm install
```

### 2. 构建前端

```bash
# 完整构建（清理 + TypeScript + SCSS）
npm run build

# 或开发构建
npm run dev

# 监听模式
npm run watch
```

### 3. 启动服务器

#### 方式一：Python 调试服务器（推荐开发使用）

```bash
# 在项目根目录
cd /home/tang/Transnet
python3 mock_server.py
```

服务器将在 http://127.0.0.1:35791 启动

#### 方式二：Rust 服务器（生产环境）

```bash
cd transnet-server
cargo run
```

### 4. 访问应用

打开浏览器访问：http://127.0.0.1:35791

## 🎨 设计系统

### 色彩

| 变量 | 值 | 用途 |
|------|-----|------|
| `--bg-void` | `#0a0a0f` | 页面背景 |
| `--accent-primary` | `#00f0ff` | 电光青，主按钮、高亮 |
| `--accent-secondary` | `#b829f7` | 等离子紫，收藏、次要 |
| `--success` | `#00ff9d` | 成功状态 |
| `--error` | `#ff3860` | 错误状态 |

### 玻璃拟态

```scss
.glass-card {
  background: rgba(26, 26, 37, 0.6);
  backdrop-filter: blur(20px) saturate(180%);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  box-shadow: 
    0 8px 32px rgba(0, 0, 0, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.1);
}
```

## 📡 API 接口

所有 API 请求以 `/api` 为前缀：

| 方法 | 路径 | 描述 |
|------|------|------|
| POST | `/translate` | 翻译文本（可匿名） |
| POST | `/account/login` | 用户登录 |
| POST | `/account/register` | 用户注册 |
| GET | `/history` | 获取历史（需登录） |
| GET | `/favorites` | 获取收藏（需登录） |
| GET | `/profile` | 获取资料（需登录） |

## 🔧 开发配置

### 环境变量 (.env)

```bash
GATEWAY_PORT=35791
GATEWAY_HOST=127.0.0.1
```

### 热重载

```bash
# 终端 1：监听 TypeScript
cd static
npm run watch:ts

# 终端 2：监听 SCSS
cd static
npm run watch:scss

# 终端 3：运行服务器
python3 mock_server.py
```

## 📝 使用说明

### 匿名翻译

1. 打开首页直接输入文本
2. 选择源语言和目标语言
3. 点击「✨ 开始翻译」
4. 翻译结果将立即显示

### 登录用户

1. 点击右上角「登录」
2. 输入邮箱和密码
3. 登录后：
   - 翻译自动保存到历史
   - 可以收藏翻译
   - 查看个人统计

### 快捷键

- `Ctrl/Cmd + Enter` - 开始翻译
- `Tab` - 切换输入框

## 🐛 调试

浏览器控制台可用对象：

```javascript
// 查看当前用户信息
AuthService.getCurrentUser()

// 检查登录状态
AuthService.isAuthenticated()

// 手动调用 API
ApiService.translate({text: "hello", source_lang: "en", target_lang: "zh"})
```

## 📦 构建部署

```bash
# 生产构建
cd static
npm run build

# 输出到 dist/ 目录，部署时只需上传：
# - dist/
# - index.html
# - resource/
```

## 🤝 贡献

1. 在 `dev/frontend` 分支开发
2. 遵循现有代码风格
3. 确保 TypeScript 类型正确
4. 测试所有功能

## 📄 License

MIT
