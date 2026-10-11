# 流萤 2D 轻侧视资源说明

当前资源以用户确认的 `docs/firefly-2d-approved.png` 为视觉基准，使用内置 imagegen 重绘为 2D Q 版轻侧视白色制服造型。保留灰银长发、青绿色发尾、蓝紫眼睛、制服领结与斜键盘；空格键靠近角色，背沿及接口朝向用户。预览中的壁纸、任务栏、放大展示和文字不属于运行素材。

`atlas.png` 为 1254×1254 RGBA PNG，保留生成工具输出的原始透明 alpha，未通过脚本抠图或重绘。角色为流萤同人插画，不是官方游戏导出素材。实际图集尺寸与请求尺寸不同，裁剪已按实际像素校准。

`character.json` 继续使用 v1：身体、左右前臂与手、独立键盘、六个表情。表情只叠加眼眉、嘴部和腮红，顺序固定为待机、眨眼、专注、开心、困倦、惊讶，提醒复用惊讶。身体含袖口，独立前臂在袖口处重叠。绘制顺序为身体/表情、键盘、左右手。

舞台为 300×320，默认 90%。身体、表情和双手共享 ±1.3 逻辑像素呼吸幅度，屏幕左手下压 3 像素、右手 2 像素，键盘固定。浅色按键气泡顶部为 216，右边距为 12，最大宽度为 276，高度为 24，均随桌宠大小缩放。图层保持源裁剪比例；六张表情使用相同裁剪尺寸与目标位置。

2026-10-10 界面优化只调整手部锚点：左前臂由 `(89,218)` 移至 `(96,202)`，右前臂由 `(149,211)` 移至 `(156,198)`，将指尖移到中部键帽，保持裁剪、等比尺寸及袖口重叠。未修改 `atlas.png`、身体、表情或键盘。角色舞台和按键气泡在实际窗口内统一下移 30 逻辑像素；窗口为 300×350，头顶提醒位于 y=2…59，整机指标条位于 y=63…87，角色头部从约 y=96 开始。清单仍保存原舞台坐标，UI 负责添加顶部偏移，配置位置使用旧舞台锚点以兼容 v1。

加载时核对 PNG 头部的 RGBA 类型与实际尺寸、清单版本、表情顺序、裁剪范围、数值有效性及呼吸/敲击的最外边界。像素解码仍由 Slint 完成。低 alpha 边缘按实际透明度合成，不把查看器突出显示的隐藏 RGB 当成不透明背景。

原生预览示例可选浅/深底色，展示属性默认为透明，正式程序不设置它；示例不采集输入、不启动管道/托盘、不写配置，不随便携包交付。实际观察及未验收项目见 `docs/VALIDATION.md`。

旧资源、清单与说明已保留在本次工作区基线备份和以前的便携包中，没有覆盖旧交付。

## 图集实际生成提示

以确认图为参考，使用内置 imagegen 与以下提示生成；没有调用 CLI/API 回退。

```text
Use case: precise-object-edit. Production asset: transparent layered sprite atlas for a 300x320 logical-pixel desktop pet. The reference is the APPROVED visual target. Recreate its LARGE LEFT character and keyboard as separate sprite parts, preserving its exact FLAT 2D anime style, light three-quarter typing angle, head tilt, white uniform, silver-gray hair with teal ends, blue-violet/pink eyes and proportions. Do not copy the background, taskbar, text, Space bubble, or small duplicate pet.

Make ONE square sprite atlas, target 2048x2048, genuinely transparent alpha everywhere outside the parts. No grid, labels, shadows on the canvas, background, text, watermark, white matte or checkerboard. Clean thin 2D outlines and simple cel shading, NOT 3D, not a figurine. All parts need clear transparent gutters and must fit fully inside canvas. Exactly TEN detached sprite objects: one body, two forearms/hands, one keyboard, six facial overlays.

BODY occupies upper-left area x=20..950 y=20..1130. A single continuous silver-gray head/hair plus torso, with a completely BLANK SKIN FACE: no eyes, eyebrows, nose, mouth or blush baked into body. Preserve the approved subtly tilted three-quarter face shape and bangs so expression overlays can be added. Hair full and flowing to muted teal tips; black headband with small turquoise/gold geometry and modest side ribbon. White sailor blouse with gray-green collar and cuffs, green ribbon and small gold clasp, gold buttons and small gold shoulder floral crest. Tiny seated waist and gray pleated skirt edge with muted gold plaid. Preserve upper arms and sleeve roots connected to shoulders, but NO FOREARMS/HANDS. End sleeves below elbows with generous overlapping fabric at the attachment so separate forearms will overlap seamlessly. Hair can conceal elbow joins. No keyboard in body. No props.

SCREEN-LEFT NEAR FOREARM/HAND detached in x=1020..1430 y=40..590: white bent sleeve continuing the body's near elbow, dark gray double-striped cuff, small natural hand with curved fingertips pressing toward bottom-right, matching the near-left typing hand in the approved reference. SCREEN-RIGHT FAR FOREARM/HAND in x=1560..1990 y=40..590: matching white sleeve, gray cuff and natural hand for far-right half of keyboard. Preserve two different perspective angles, not mirrored identical fists. No brown or armored sleeves. Both arms must have overlapping fabric margins for body attachment.

KEYBOARD detached in x=990..2020 y=650..1120. Match approved reference's compact ivory/mint three-row blank keyboard and diagonal orientation: left end lower on canvas, right end higher, about 25 degrees. We see the REAR/USB edge closest to viewer, with small socket and tiny short cable. IMPORTANT: keyboard's single long SPACEBAR belongs on FAR edge toward the character, not near spectator. Three rows of blank square keys, little housing thickness. Render in flat 2D line art matching body. No hands on this detached keyboard, no floor shadow.

SIX EXPRESSION OVERLAYS in uniform 3-column by 2-row cells across bottom half: column centers x=350,1024,1698, row centers y=1370,1820. Each expression fits within a 560x320 transparent rectangle around its center. These are ONLY eyes, eyebrows, tiny mouth and blush with alpha around them. Absolutely NO skin oval, face, ears, hair, head or backing patch. Same exact eye and mouth registration and perspective in all six. Their eye line rises mildly toward screen-right, matching the head tilt and slight three-quarter view of the approved face; left eye slightly closer/larger. Eyes blue-violet with lavender and rose highlights, NOT solid turquoise. First row left: idle open eyes gentle closed-mouth smile; middle: blink closed eyes same mouth; right: focused open eyes slightly concentrated brows. Second row left: happy smiling curved closed eyes and small cheerful open mouth; middle: sleepy half-lidded eyes and tiny yawn; right: surprised rounded eyes and tiny open mouth. Keep all six expression overlays identical overall scale and stable eyes/mouth anchors, no drifting between cells. They will be cropped and superimposed on the blank body.

Keep original approved character design and compact typing silhouette, no large wing ornament, no brown corset/cape, no full-body standing figure, no decorative effects. Every part isolated with alpha and enough margin; don't overlap any two atlas objects.
```
