# 无背景极简 SVG 图标集

高级感透明底版本，去除了背景方块，采用纯文字+线性符号设计，适配任何背景色（深色/浅色/渐变主题都能用），风格简单现代，适合编辑器、文件管理器、个人博客/项目使用。

## ✨ 特点
- 🫥 完全透明背景，无任何底色，适配所有主题
- ✒️ 统一极简风格：语言类用中粗品牌色文字，符号类用2px圆角线条
- 📐 16×16标准矢量尺寸，线条清晰，小尺寸不糊
- 🎨 颜色对应各语言官方品牌色，低饱和度更高级
- ⚖️ MIT协议，完全开源免费，无版权风险
- 📦 单文件独立，可直接内联使用

## 📋 包含内容
共36个图标：
- 语言类：Python、JavaScript、TypeScript、Rust、Go、Java、C、C++、C#、F#、YAML、Markdown、TOML、Shell、Ruby、PHP、Swift、Kotlin、Docker、Vue、React、Svelte、Zig、Dart、R、Lua、Haskell、Elixir、Julia、Scala、Perl、Clojure、Erlang
- 符号类（线性设计）：
  - HTML：尖括号 `<>` 线条
  - CSS：井号 `#` 线条
  - JSON：大括号 `{}` 线条

## 🚀 使用方法
1. 直接在HTML中引用：
```html
<img src="./icons/python.svg" alt="Python" width="20" height="20">
```

2. 内联到代码中，可通过CSS控制颜色：
```html
<svg class="lang-icon" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="20" height="20">
  <text x="12" y="17" font-family="system-ui" font-size="12" font-weight="600" text-anchor="middle" fill="currentColor">Py</text>
</svg>
```

## 📝 自定义修改
- 文字类：修改`font-size`调整大小，修改`fill`换色，修改`font-weight`调整字重
- 线条类：修改`stroke-width`调整线条粗细，修改`stroke`换色
- 所有图标都是标准SVG，可直接在Figma/Illustrator中编辑

## 📄 协议
MIT License，完全免费，可用于任何个人或商业项目。
