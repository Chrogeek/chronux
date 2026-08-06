# Chronux DSL 语法简要说明

**文档对应 Chronux 版本**：0.9

## **⚠注意**
Chronux 仍处于持续的迭代过程中，随时可能引入 breaking change，本文档仅供参考。

本文档仅介绍了通用的动画制作相关的语法，不涉及特殊扩展。

## 基本语法与文件格式

Chronux DSL 需要用到两种基本语言：XML (SVG) 和 TOML。

### 静态图元资产

使用 SVG 语法，建议使用 Inkscape 编辑。

需要注意的是，Chronux 目前要求将图元的样式**以 style 属性直接内联**在图元对应的 SVG 标签中。将样式直接写成元素属性而不放在 style 属性下，或者用 `<style>` 标签嵌入/外链的 CSS **均不会被解析**。

下面是一个正确的例子。

```svg
<circle
    style="display:inline;fill:#ff0000;stroke:#ffffff;stroke-width:3;stroke-linecap:round;stroke-linejoin:round;stroke-dasharray:none;paint-order:normal"
    id="path2"
    cx="40"
    cy="40"
    r="10"
    inkscape:label="element-name"
/>
```

如果图元文件是完全通过 Inkscape 创作的，那么 Inkscape 默认就会按照上述的正确方式保存，无需特殊操作。如果是通过其他矢量绘图软件（例如 Illustrator），那么你可能需要手动转换样式的写法。

Chronux 对某些属性有特殊要求，并且也引入了一些 Chronux 自定义的属性，参见后文说明。

### 动画配置文件

使用 TOML 语法描述，具体的属性定义参见后文说明。

## 动画基本概念

### 动画类型

Chronux 目前支持如下种类的动画，以及每种动画各自所需的参数：

```rs
pub enum Animation {
    #[serde(rename = "none")]
    None,  // 无动作，可用于占位代码
    #[serde(rename = "trim-path")]
    TrimPath {  // 修剪路径（进入）
        #[serde(default)]
        reverse: bool,  // 是否反向
    },
    #[serde(rename = "trim-path-out")]
    TrimPathOut {  // 修剪路径（退出）
        #[serde(default)]
        reverse: bool,  // 是否反向
    },
    #[serde(rename = "typewriter")]
    Typewriter {  // 打字机（逐字显示一段文本）
        #[serde(default)]
        reverse: bool,  // 是否反向
    },
    #[serde(rename = "typewriter-out")]
    TypewriterOut {  // 打字机（退出，逐字消去文本）
        #[serde(default)]
        reverse: bool,  // 是否反向
    },
    #[serde(rename = "sweep")]
    Sweep,  // 扇形展开（进入）
    #[serde(rename = "sweep-out")]
    SweepOut,  // 扇形消失（退出）
    #[serde(rename = "tween")]
    Tween(Tween),  // 属性过渡；参数列表需指定各种属性需要过渡到的目标值（不需要书写变化之前的值）
    #[serde(rename = "fade-in")]
    FadeIn,  // 淡入（进入）
    #[serde(rename = "fade-out")]
    FadeOut,  // 淡出（退出）
    #[serde(rename = "move")]
    Move { dx: f32, dy: f32 },  // 移动图元
    #[serde(rename = "stretch-in")]
    StretchIn,  // 从大小为零的状态拉伸图元（进入）
    #[serde(rename = "stretch-out")]
    StretchOut,  // 将图元的大小缩小到零（退出）
    #[serde(rename = "pie-in")]
    PieIn,  // 从图元中心点向外以圆形的方式扩散（进入）
    #[serde(rename = "pie-out")]
    PieOut,  // 将图元以圆形方式收缩到中心点，直至消失（退出）
}
```

### 图元的名称和查找

Chronux 支持通过如下方式查找图元，它们可以用在事件的 `object` 中：
* 图元的 `id` 属性
* 图元的 `inkscape:label` 属性
* 图元及其父图元各自的 `inkscape:label` 属性使用 `/` 字符连接起来：`parent-label/child-label`，注意只能使用 1 次 `/`，不能级联

⚠注意：
* `inkscape:label` 属性优先于 `id` 属性。在 Inkscape 中编辑图元时，可以通过双击对象列表为其重命名，即可填充 `inkscape:label`。
* Chronux 不会检查 `inkscape:label` 是否有重复。重复时，只会选中其中一个图元。
* 图元名称原则上可以使用各种字符（Chronux 并不做检查），但**下列符号不建议使用**：
  * `/`：由于支持 `parent-label/child-label` 语法指定图元，为了防止二义性，不建议使用；
  * `(`、`)`、`#`：Chronux 支持的一些用于特殊表达的字符。

对于修剪路径（`trim-path` 和 `trim-path-out`）动画，还支持一种特殊的图元名称——联合路径，可以将多个路径捆绑在一起形成一个临时的路径组（复合路径）。语法如下：

```bcnf
path-group ::= atom | parallel | sequential

parallel ::= '#par(' comma-separated-path-groups ')'

sequential ::= '#seq(' comma-separated-path-groups ')'

atom ::= inkscape-label | svg-id

comma-separated-path-groups ::= 用逗号分隔的多个 path-group
```

注意目前 Chronux 对于 `comma-separated-path-groups` 中出现多余逗号（包括空参数或者末尾的 trailing comma）没有进行特殊处理，因此目前不建议这样书写。

* 对于 `parallel`，其中每个参数指定的路径将被**平行**地修剪：起始、结束时间均相同；通过 `parallel` 指定的复合路径的长度等于每个子路径的平均值；
* 对于 `sequential`，其中每个参数指定的路径将被**首尾相接**作为一个长路径修剪，各个子路径将**按照长度比例分配**动画的时间；通过 `sequential` 指定的复合路径的长度等于每个子路径的长度之和。

## 事件声明语法

Chronux 采用基于时序关系的事件驱动模型：每个图元的每次动画分别称为一个**事件**（Event），每个事件就对应 TOML 中的一个表。

### 基本事件（events）

```toml
[[events]]
id = "事件的唯一标识符"
depends = ["依赖的事件ID列表"]
offset = 在依赖事件之后相隔多少秒开始本事件
duration = 动画持续时间（秒）
easing = "使用何种缓动函数"
object = "作用于哪个图元"
animation = "动画类型"
...特定于animation指定的动画类型的参数
```

上面的 `offset` 表示在 `depends` 中列出的所有依赖事件**全部结束**后，再经过多少秒，开始执行本事件。

还有两种指定时序关系的方式：

* `delay`：表示在 `depends` 中列出的所有依赖事件**全部开始**后，到本事件开始执行，中间间隔的时间。
* `track`：要求 `depends` 依赖列表恰好包含 1 个事件，且该事件的动画类型 `animation` 必须是 `trim-path` 或 `trim-path-out`。表示追踪被依赖的修剪路径动画当前进行到了什么位置（也就是被修剪的路径的活动端点的坐标），然后选择追踪位置距离本事件的 `object` 指定的图元**最近**的那个时刻，然后再加上由 `track` 指定的偏移时间，作为本事件的开始时间。
* 如果 `depends` 为空，则时序关系的基准值为 0，也就是事件将在 `offset` 或 `delay` 指定的时刻开始。

### 事件组（groups）

很多场景需要让多个事件同步执行（同时开始且同时结束），因此可以使用 `groups` 来简化书写：

```toml
[[groups]]
id = "事件组的名称"
depends
offset
duration
easing
anilist = [
    { object, animation, ... },
]
```

相比单个事件的语法，有如下需要说明的细节：

* `anilist` 的每个元素都是一个事件，但不包含 `id`、`depends`、`offset`、`duration` 属性；
* `anilist` 中的每个事件可以额外指定 `easing` 属性，表示覆盖 `[[groups]]` 中定义的统一使用的缓动函数；
* 事件组的名称 `id` 将被用于 `anilist` 的**第一个**事件（因此可以用于其他 `events` 或 `groups` 的依赖列表）；列表中其他事件的名称由 Chronux 管理，对用户不可见。

### 追踪组（trackers）

修剪路径动画经常用于这样一种场景：随着路径的生长，它的端点每触碰到某些特定位置，就执行一段动画。由于这些动画并不是同步的，因此 `groups` 的表达能力受限，所以引入了特殊的追踪组（trackers）：

```toml
[[trackers]]
id
depends
offset
duration
easing
object
animation = "trim-path"  # 或者 trim-path-out
children = [
    child1,
    child2,
    ...
]
```

`children` 中的每个元素的语法同 `groups`，只是没有 `depends` 属性（它们自动依赖 `trackers` 定义的路径修剪动画本身），并且其中指定时序关系的方式通常是 `track`。相当于将多个 `groups` 合并成了一个大的 `trackers`。

`trackers` 实质为下面代码的语法糖，二者完全等价，Chronux 目前的实现方式也是如此展开：

```toml
[[events]]
id = "tracked-event"
depends
offset
duration
easing
object
animation = "trim-path"  # 或者 trim-path-out

[[groups]]
...child1
depends = ["tracked-event"]

[[groups]]
...child2
depends = ["tracked-event"]

...
```

## 镜头操作（viewports）

使用 `viewports` 来描述镜头的移动：

```toml
[[viewports]]
tracked-path = "镜头沿着哪个路径移动"
keyframes = [
    { depends = "关联事件", delay, width, x, y },
    ...
]
```

* 需要先在 SVG 中画出镜头中心点的移动路径，然后沿着这个路径移动。
* `keyframes` 定义了镜头参数的关键帧：
  * `depends` 和 `delay`（也可以使用 `offset`）指定时间参数，表示相对哪个事件偏移多少时间；
  * `x`、`y` 表示镜头中心点的坐标，可以不是 `tracked-path` 上的点，Chronux 将会尝试求解 `tracked-path` 上距离 `(x, y)` 最近的位置；
  * `width` 表示镜头的宽度，最终渲染出来的画面只会包含镜头内的图元。
* 对于 `keyframes` 中的第一个和最后一个关键帧，不需要指定 `x` 和 `y`，Chronux 将使用 `tracked-path` 的起终点来填充；
* 对于 `keyframes` 中的第一个关键帧，必须指定 `width`；后续的关键帧可以不指定 `width`，此时将会使用上一个关键帧的 `width`；
* 关键帧必须按照时间顺序书写，且每个关键帧的位置必须沿着 `tracked-path` 按顺序排列，Chronux 会检查并且在违反这一条时报错；
* 各个关键帧之间的坐标和镜头大小使用 PCHIP 插值，它具有以下特点：
  * 镜头只会沿着 `tracked-path` 的路径方向移动，**不会倒退**；
  * 起点和终点处的镜头瞬时移动速度为 0；
  * 镜头大小的变化方向和关键帧之间确定的变化方向一致；
  * 在任意时刻，镜头的参数变化都是连续且平滑的。
* 在两个 `viewports` 各自时间段之间的间隙，镜头参数采用线性插值。

## 资产管理、HUD、视频输出

```toml
[input]
version = "Chronux 版本"
svg = "图元资产 SVG 文件名"
hud = "HUD 图元资产 SVG 文件名"
bgm = "背景音乐文件名"
```

可以额外指定一个 `hud` 文件，其中的图元不会随着镜头移动，而是直接作为叠加层（HUD）绘制在视频画面上。

Chronux 总是假定 HUD 文件的宽度为 1920 像素，渲染时会缩放到正确的大小。

```toml
[output]
width = 3840
height = 2160
fps = 60
background-color = "white"
# time-start
# time-end
```

可以指定最终渲染视频的分辨率、帧率、背景颜色。

对于预览某个片段的需求，可以使用 `time-start` 和 `time-end` 指定一个时间区间（单位为秒），Chronux 将只会渲染这一段区间内的动画视频。

## 最佳实践

虽然 Chronux 会自动整理事件的时序关系，因此 TOML 语法中可以按照任意顺序排列事件，但是我们仍然建议按照动画的逻辑顺序书写事件。

我们建议将同一个事件对应的（一个或多个）`events`、`groups`、`trackers`、`viewports` 都放在一起，因为这样在检查动画是否正确时不容易出现遗漏。

**注意**：这里利用了 TOML 的表数组（array of tables）可以分散书写的特点，这也是 Chronux DSL 选择 TOML 作为基础语法的原因之一。这种写法本身并不是 TOML 的最佳实践，但在 Chronux DSL 中并非如此（某些 TOML 格式化工具在默认状态下会将同一个表数组聚集在一起，这样会将同一个动画的多个表打散，反而不利于阅读和代码审计）。
