import org.codehaus.groovy.ast.CodeVisitorSupport
import org.codehaus.groovy.ast.builder.AstBuilder
import org.codehaus.groovy.ast.expr.*
import org.codehaus.groovy.ast.stmt.*
import org.codehaus.groovy.control.CompilePhase
import groovy.json.JsonOutput

// Each Groovy closure compiles as a separate method. Count it independently.
class BranchCount extends CodeVisitorSupport {
    int score = 1
    void visitClosureExpression(ClosureExpression e) {}
    void visitIfElse(IfStatement s) { score++; super.visitIfElse(s) }
    void visitForLoop(ForStatement s) { score++; super.visitForLoop(s) }
    void visitWhileLoop(WhileStatement s) { score++; super.visitWhileLoop(s) }
    void visitDoWhileLoop(DoWhileStatement s) { score++; super.visitDoWhileLoop(s) }
    void visitCaseStatement(CaseStatement s) { score++; super.visitCaseStatement(s) }
    void visitCatchStatement(CatchStatement s) { score++; super.visitCatchStatement(s) }
    void visitTernaryExpression(TernaryExpression e) { score++; super.visitTernaryExpression(e) }
    void visitBinaryExpression(BinaryExpression e) {
        if (e.operation.text in ['&&', '||']) score++
        super.visitBinaryExpression(e)
    }
}
class ClosureScores extends CodeVisitorSupport {
    List rows = []
    void visitClosureExpression(ClosureExpression e) {
        def count = new BranchCount()
        e.code.visit(count)
        rows.add([line:e.lineNumber, complexity:count.score])
        super.visitClosureExpression(e)
    }
}
def result = args.collect { name ->
    def visitor = new ClosureScores()
    def nodes = new AstBuilder().buildFromString(CompilePhase.CONVERSION, false, new File(name).text)
    nodes.findAll { it instanceof BlockStatement }.each { it.visit(visitor) }
    def methods = nodes.findAll { it instanceof org.codehaus.groovy.ast.ClassNode && !it.script }
        .collectMany { it.methods }.findAll { it.lineNumber > 0 }.collect { method ->
            def count = new BranchCount()
            method.code.visit(count)
            [name:method.name, line:method.lineNumber, complexity:count.score]
        }
    [file:name, closures:visitor.rows, methods:methods]
}
println JsonOutput.prettyPrint(JsonOutput.toJson([tool:"Groovy ${GroovySystem.version} compiler AST", results:result]))
