#include "ProtocolFlowGraph.hpp"

namespace {
struct All : public ModulePass {
    static char ID;

    All() : ModulePass(ID) {}

    // Module表示一个完整的LLVM IR文件（通常对应一个源文件）
    bool runOnModule(Module &M) override {
        auto graphs = initDetectedGraphs(M, true, false);           // 会调用PFGraph::build进行协议图构建
        auto results = json::Object();
        auto main_comp = extractMainComp(&M);
        for (auto g : graphs) {
            auto g_result = json::Object();
            auto report = json::Object();
            report["uco"] = g->detectUnconstrainedOutput();         // 1. 未约束输出
            report["usci"] = g->detectUnconstrainedCompInput();     // 2. 未约束组件输入
            report["dcd"] = g->detectDataflowConstraintDis();       // 3. 数据流约束不一致
            report["usco"] = g->detectUnusedCompOutput();           // 4. 未使用组件输出
            report["us"] = g->detectUnusedSignal();                 // 5. 未使用信号
            report["dbz"] = g->detectDivideByZeroUnsafe();          // 6. 除零约束
            report["ndd"] = g->detectNondeterministicDataflow();    // 7. 非确定性数据流
            report["tm"] = g->detectTypeMismatch();                 // 8. 类型不匹配
            report["am"] = g->detectAssignmentMisuse();             // 9. 赋值误用
            g_result["reports"] = obj2value(report);
            results[g->getName()] = obj2value(g_result);
        }
        json::OStream J(errs());
        J.value(obj2value(results));
        errs() << "\n";
        return false;
    };
};
}  // namespace

char All::ID = 0;
static RegisterPass<All> X(
    "All",
    "Detect whether every component is checked or not.",
    false, false);
