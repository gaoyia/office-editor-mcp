# ⚠️ Normal继承6路径核实（原PR已有问题）

- [x] 原始2族×3路径，4源SHA/缓存/actualcells校验，不按6新功能计数。
- [x] 非零styleXF和builtin0的规范链、Normal字体与cell实际字体分开，库96几何6/6验证。
- [x] 17组有限控制公开COM：源Arial11，Office NormalCalibri11；两自有SaveCopyAs未重开，ZIP证实Normal字体语义改变，不是name/index变化。
- [x] 当前6个CLI输出与既有after字节一致，原gold不动；不改renderer、不font0/7/8硬回退。
- [x] 新验收工具和4回归，加现19共23 Python；无Rust改动，原1868日志不冒称重跑。
- [ ] Office同语义可比证据仍不足，DPI豁免保持38/49，不将此6强判仅DPI/库bug/全图PASS。

精确缺证：公开兼容规则说明此载入字体语义改写，或Office能保留非零Normal-font引用的独立同语义合法文件。现仅证明本探针族现象，停止扩探针，不研究AA/legend/5 Export边界，不恢复十项功能。

证据：桌面office-editor-PR10-Normal继承核实-20261117-151855；normal-inheritance-certificates.json/six-path-certificates.csv/finite-control-results.csv/index/README。官方规范解释：https://learn.microsoft.com/en-us/answers/questions/1110580/where-is-the-normal-style-font-defined-in-an-xlsx 。
