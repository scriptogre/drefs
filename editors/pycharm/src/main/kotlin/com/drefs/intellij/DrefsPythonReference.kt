package com.drefs.intellij

import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFileSystemItem
import com.intellij.psi.PsiReferenceBase
import com.intellij.psi.util.QualifiedName
import com.jetbrains.python.psi.PyClass
import com.jetbrains.python.psi.PyFile
import com.jetbrains.python.psi.PyFunction
import com.jetbrains.python.psi.PyPsiFacade
import com.jetbrains.python.psi.PyStringLiteralExpression
import com.jetbrains.python.psi.PyUtil
import com.jetbrains.python.psi.resolve.PyQualifiedNameResolveContext

/**
 * A reference from a docstring cross-reference segment to a Python symbol.
 *
 * Uses PyCharm's own resolution — the same code path as import statements.
 * Pattern taken from JetBrains' PyDocumentationLink.kt.
 */
class DrefsPythonReference(
    element: PyStringLiteralExpression,
    range: TextRange,
    private val qualifiedName: String,
    private val resolveShort: Boolean = false,
) : PsiReferenceBase<PyStringLiteralExpression>(element, range, /* soft = */ false) {

    override fun resolve(): PsiElement? {
        if (resolveShort) {
            return resolveShortName()
        }
        return resolveQualified()
    }

    private fun resolveShortName(): PsiElement? {
        val project = element.project
        val facade = PyPsiFacade.getInstance(project)
        val resolveContext = facade.createResolveContextFromFoothold(element)
        val withMembers = resolveContext.copyWithMembers()

        // Get the containing file to check its imports.
        val pyFile = element.containingFile as? PyFile ?: return null

        // 1. Check imports in the file.
        for (imp in pyFile.fromImports) {
            for (importedName in imp.importElements) {
                val visibleName = importedName.asName ?: importedName.importedQName?.lastComponent ?: continue
                if (visibleName == qualifiedName) {
                    val fqn = importedName.importedQName?.toString() ?: continue
                    val qName = QualifiedName.fromDottedString(fqn)
                    return facade.resolveQualifiedName(qName, withMembers).firstOrNull()
                        ?: resolveClassMember(facade, qName, withMembers)
                }
            }
        }

        // 2. Check definitions in the file.
        for (cls in pyFile.topLevelClasses) {
            if (cls.name == qualifiedName) return cls
        }
        for (func in pyFile.topLevelFunctions) {
            if (func.name == qualifiedName) return func
        }
        for (target in pyFile.topLevelAttributes) {
            if (target.name == qualifiedName) return target
        }

        return null
    }

    private fun resolveQualified(): PsiElement? {
        val project = element.project
        val facade = PyPsiFacade.getInstance(project)
        val qName = QualifiedName.fromDottedString(qualifiedName)
        val resolveContext = facade.createResolveContextFromFoothold(element)

        // Try as module/package first.
        val moduleResult = facade.resolveQualifiedName(qName, resolveContext)
            .asSequence()
            .filterIsInstance<PsiFileSystemItem>()
            .map { PyUtil.turnDirIntoInit(it) }
            .filterIsInstance<PyFile>()
            .firstOrNull()
        if (moduleResult != null) return moduleResult

        // Try as top-level function/class with member access.
        val withMembers = resolveContext.copyWithMembers()
        val directResult = facade.resolveQualifiedName(qName, withMembers).firstOrNull()
        if (directResult != null) return directResult

        // Try as a class method/attribute.
        return resolveClassMember(facade, qName, withMembers)
    }

    private fun resolveClassMember(
        facade: PyPsiFacade,
        qName: QualifiedName,
        withMembers: PyQualifiedNameResolveContext,
    ): PsiElement? {
        if (qName.componentCount > 1) {
            val parentQName = qName.removeLastComponent()
            val parentResult = facade.resolveQualifiedName(parentQName, withMembers)
                .filterIsInstance<PyClass>()
                .firstOrNull()
            if (parentResult != null) {
                val method = parentResult.findMethodByName(qName.lastComponent, true, null)
                if (method != null) return method
                val attr = parentResult.findClassAttribute(qName.lastComponent.orEmpty(), true, null)
                if (attr != null) return attr
            }
        }
        return null
    }
}
