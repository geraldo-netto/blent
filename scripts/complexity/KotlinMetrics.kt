import java.io.File
import org.jetbrains.kotlin.cli.jvm.compiler.EnvironmentConfigFiles
import org.jetbrains.kotlin.cli.jvm.compiler.KotlinCoreEnvironment
import org.jetbrains.kotlin.config.CompilerConfiguration
import org.jetbrains.kotlin.com.intellij.openapi.util.Disposer
import org.jetbrains.kotlin.com.intellij.psi.PsiElement
import org.jetbrains.kotlin.com.intellij.psi.PsiErrorElement
import org.jetbrains.kotlin.com.intellij.psi.util.PsiTreeUtil
import org.jetbrains.kotlin.lexer.KtTokens
import org.jetbrains.kotlin.psi.*

// Independently expressed count; see README.md for the pinned language rules.
fun increment(element: PsiElement): Int = when (element) {
    is KtNamedFunction -> if (element.hasBody() && element.name != null) 1 else 0
    is KtIfExpression, is KtLoopExpression, is KtWhenEntry -> 1
    is KtBinaryExpression -> if (element.operationToken in setOf(KtTokens.ANDAND, KtTokens.OROR)) 1 else 0
    else -> 0
}

fun complexity(element: PsiElement): Int = increment(element) + element.children.sumOf(::complexity)

fun initializationComplexity(element: PsiElement): Int =
    if (element is KtNamedFunction || element is KtPropertyAccessor || element is KtClassOrObject) 0
    else increment(element) + element.children.sumOf(::initializationComplexity)

fun initializationBodies(declarations: List<KtDeclaration>): List<PsiElement> = declarations.mapNotNull {
    when (it) {
        is KtAnonymousInitializer -> it.body
        is KtProperty -> if (propertyInitialization(it)) it.delegateExpression ?: it.initializer else null
        else -> null
    }
}

fun primaryBodies(element: KtClassOrObject): List<PsiElement> {
    val parameters = element.primaryConstructor?.valueParameters.orEmpty().mapNotNull { it.defaultValue }
    return parameters + element.superTypeListEntries + initializationBodies(element.declarations)
}

fun objectBodies(element: KtObjectDeclaration, name: String): List<PsiElement> = when {
    element.isObjectLiteral() -> primaryBodies(element)
    name == "<init>" -> element.superTypeListEntries
    else -> initializationBodies(element.declarations)
}

fun secondaryBodies(element: KtSecondaryConstructor): List<PsiElement> {
    val own = listOfNotNull(element.getDelegationCall(), element.bodyExpression) +
        element.valueParameters.mapNotNull { it.defaultValue }
    val parent = element.getContainingClassOrObject()
    return if (parent.hasPrimaryConstructor() || element.getDelegationCall().isCallToThis) own
        else own + initializationBodies(parent.declarations)
}

fun classBodies(element: KtClassOrObject, name: String): List<PsiElement> = when {
    element is KtEnumEntry -> initializationBodies(element.declarations)
    name == "<clinit>" -> element.declarations.filterIsInstance<KtEnumEntry>()
        .flatMap { it.superTypeListEntries }
    else -> primaryBodies(element)
}

fun methodBodies(element: PsiElement, name: String): List<PsiElement> = when (element) {
    is KtNamedFunction -> listOfNotNull(element.bodyExpression)
    is KtPropertyAccessor -> listOfNotNull(element.bodyExpression)
    is KtSecondaryConstructor -> secondaryBodies(element)
    is KtLambdaExpression -> listOfNotNull(element.bodyExpression)
    is KtObjectDeclaration -> objectBodies(element, name)
    is KtFile -> initializationBodies(element.declarations)
    is KtClassOrObject -> classBodies(element, name)
    else -> emptyList()
}

fun methodComplexity(element: PsiElement, name: String): Int =
    if (element is KtNamedFunction && element.name != null) complexity(element)
    else if (name == "<init>" || name == "<clinit>")
        1 + methodBodies(element, name).sumOf(::initializationComplexity)
    else 1 + methodBodies(element, name).sumOf(::complexity)

fun reportMethod(source: String, path: String, element: PsiElement) {
    val line = source.substring(0, element.textOffset).count { it == '\n' } + 1
    for (name in listOfNotNull(functionName(element), additionalInitialization(element))) {
        println("$path\t$line\t$name\t${methodComplexity(element, name)}")
    }
}

fun report(factory: KtPsiFactory, path: String) {
    val source = File(path).readText()
    val file = factory.createFile(File(path).name, source)
    check(PsiTreeUtil.findChildOfType(file, PsiErrorElement::class.java) == null) { "$path: Kotlin parse error" }
    for (element in PsiTreeUtil.collectElements(file) {
        functionName(it) != null || additionalInitialization(it) != null
    }) {
        reportMethod(source, path, element)
    }
}

fun main(paths: Array<String>) {
    val disposable = Disposer.newDisposable()
    try {
        val environment = KotlinCoreEnvironment.createForProduction(disposable,
            CompilerConfiguration(), EnvironmentConfigFiles.JVM_CONFIG_FILES)
        val factory = KtPsiFactory(environment.project, false)
        paths.forEach { report(factory, it) }
    } finally { Disposer.dispose(disposable) }
}
