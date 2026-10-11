import org.jetbrains.kotlin.com.intellij.psi.PsiElement
import org.jetbrains.kotlin.psi.*
import org.jetbrains.kotlin.lexer.KtTokens

// Shared source-method ownership; coverage and complexity retain independent counters.
// JVM default null fields emit no assignment; empty blocks emit no body.
// Keep parameter defaults, calls, delegates and every nonempty init body.
fun executableInitializer(expression: KtExpression): Boolean = when (expression) {
    is KtParenthesizedExpression -> expression.expression?.let(::executableInitializer) == true
    is KtConstantExpression -> expression.text != "null"
    else -> true
}

fun propertyInitialization(property: KtProperty): Boolean {
    if (property.hasModifier(KtTokens.CONST_KEYWORD)) return false
    return property.hasDelegateExpression() || property.initializer?.let(::executableInitializer) == true
}

fun hasInitialization(declarations: List<KtDeclaration>): Boolean = declarations.any {
    when (it) {
        is KtAnonymousInitializer -> (it.body as? KtBlockExpression)?.statements?.isNotEmpty() == true
        is KtProperty -> propertyInitialization(it)
        else -> false
    }
}

fun instanceInitialization(element: KtClassOrObject, body: Boolean, base: Boolean): String? {
    if (!element.hasPrimaryConstructor()) return null
    return if (element.hasExplicitPrimaryConstructor() || body || base) "<init>" else null
}

fun classInitialization(element: KtClassOrObject): String? {
    if (element is KtEnumEntry) return null // Entry arguments execute in the enum's static method.
    if (element is KtClass && (element.isInterface() || element.isAnnotation())) return null
    val body = hasInitialization(element.declarations)
    val base = element.superTypeListEntries.any { it is KtSuperTypeCallEntry }
    if (element is KtObjectDeclaration) {
        if (!body && !base) return null
        return if (element.isObjectLiteral()) "<init>" else "<clinit>"
    }
    // Shared property/init code in secondary-only classes is already part of
    // every authored secondary constructor's native method counters.
    return instanceInitialization(element, body, base)
}

fun namedFunction(element: KtNamedFunction): String? =
    if (element.hasBody()) element.name ?: "<anonymous>" else null

fun accessor(element: KtPropertyAccessor): String? =
    if (element.hasBody()) (if (element.isGetter) "get:" else "set:") + element.property.name else null

fun fileInitialization(element: KtFile): String? =
    if (hasInitialization(element.declarations)) "<clinit>" else null

fun additionalInitialization(element: PsiElement): String? = when (element) {
    is KtEnumEntry -> if (hasInitialization(element.declarations)) "<init>" else null
    is KtClass -> if (element.isEnum()) "<clinit>" else null
    is KtObjectDeclaration -> if (!element.isObjectLiteral() &&
        element.superTypeListEntries.any { it is KtSuperTypeCallEntry }) "<init>" else null
    else -> null
}

fun functionName(element: PsiElement): String? = when (element) {
    is KtNamedFunction -> namedFunction(element)
    is KtPropertyAccessor -> accessor(element)
    is KtSecondaryConstructor -> "<init>"
    is KtLambdaExpression -> "<lambda>"
    is KtClassOrObject -> classInitialization(element)
    is KtFile -> fileInitialization(element)
    else -> null
}

