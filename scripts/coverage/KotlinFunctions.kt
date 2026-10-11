import java.io.File
import org.jetbrains.kotlin.cli.jvm.compiler.EnvironmentConfigFiles
import org.jetbrains.kotlin.cli.jvm.compiler.KotlinCoreEnvironment
import org.jetbrains.kotlin.config.CompilerConfiguration
import org.jetbrains.kotlin.com.intellij.openapi.util.Disposer
import org.jetbrains.kotlin.com.intellij.psi.PsiElement
import org.jetbrains.kotlin.com.intellij.psi.PsiErrorElement
import org.jetbrains.kotlin.com.intellij.psi.util.PsiTreeUtil
import org.jetbrains.kotlin.psi.*
import org.jetbrains.kotlin.lexer.KtTokens

fun reportFunction(source: String, path: String, element: PsiElement) {
    val names = listOfNotNull(functionName(element), additionalInitialization(element))
    if (names.isEmpty()) return
    val first = source.substring(0, element.textRange.startOffset).count { it == '\n' } + 1
    val last = source.substring(0, element.textRange.endOffset).count { it == '\n' } + 1
    val statement = (element as? KtLambdaExpression)?.bodyExpression?.statements?.firstOrNull()
    val body = statement?.let { source.substring(0, it.textRange.startOffset).count { c -> c == '\n' } + 1 } ?: first
    names.forEach { name -> println("$path\t$first\t$last\t$name\t$body") }
}

fun inventory(factory: KtPsiFactory, path: String) {
    val source = File(path).readText()
    val file = factory.createFile(File(path).name, source)
    check(PsiTreeUtil.findChildOfType(file, PsiErrorElement::class.java) == null) { "$path: Kotlin parse error" }
    for (element in PsiTreeUtil.collectElements(file) {
        functionName(it) != null || additionalInitialization(it) != null
    }) {
        reportFunction(source, path, element)
    }
}

fun main(paths: Array<String>) {
    val disposable = Disposer.newDisposable()
    try {
        val environment = KotlinCoreEnvironment.createForProduction(disposable,
            CompilerConfiguration(), EnvironmentConfigFiles.JVM_CONFIG_FILES)
        val factory = KtPsiFactory(environment.project, false)
        paths.forEach { inventory(factory, it) }
    } finally { Disposer.dispose(disposable) }
}
