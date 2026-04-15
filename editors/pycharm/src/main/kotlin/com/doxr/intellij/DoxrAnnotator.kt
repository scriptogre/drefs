package com.doxr.intellij

import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.Annotator
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.util.TextRange
import com.intellij.psi.PsiElement
import com.jetbrains.python.psi.PyStringLiteralExpression
import java.util.regex.Pattern

/**
 * Highlights cross-reference dotted paths inside docstrings to look like
 * code identifiers rather than plain docstring text.
 */
class DoxrAnnotator : Annotator {

    companion object {
        private val MKDOCS_EXPLICIT = Pattern.compile("\\[[^\\]]*\\]\\[([a-zA-Z_][\\w.]*)\\]")
        private val MKDOCS_AUTOREF = Pattern.compile("\\[([a-zA-Z_][\\w.]*)\\]\\[\\]")
        private val SPHINX_XREF = Pattern.compile(
            ":(class|func|meth|mod|attr|exc|data|obj|const|type):`~?([^`]+)`"
        )
        private val DOXR_NATIVE = Pattern.compile("\\[`?([a-zA-Z_][\\w.]*)`?\\]")
    }

    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        val stringLiteral = element as? PyStringLiteralExpression ?: return
        val text = stringLiteral.text
        if (!text.startsWith("\"\"\"") && !text.startsWith("'''")) return

        val content = stringLiteral.stringValue
        val contentOffset = stringLiteral.stringValueTextRanges.firstOrNull()?.startOffset ?: return
        val elementOffset = stringLiteral.textRange.startOffset

        // Track offsets from MkDocs/Sphinx patterns for dedup.
        val existingOffsets = mutableSetOf<Int>()

        collectOffsets(MKDOCS_EXPLICIT, content, 1, existingOffsets)
        collectOffsets(MKDOCS_AUTOREF, content, 1, existingOffsets)
        collectOffsets(SPHINX_XREF, content, 2, existingOffsets)

        highlightRefs(MKDOCS_EXPLICIT, content, 1, contentOffset, elementOffset, holder)
        highlightRefs(MKDOCS_AUTOREF, content, 1, contentOffset, elementOffset, holder)
        highlightRefs(SPHINX_XREF, content, 2, contentOffset, elementOffset, holder)

        highlightNativeRefs(content, contentOffset, elementOffset, existingOffsets, holder)
    }

    private fun collectOffsets(
        pattern: Pattern,
        content: String,
        group: Int,
        out: MutableSet<Int>,
    ) {
        val matcher = pattern.matcher(content)
        while (matcher.find()) {
            var path = matcher.group(group) ?: continue
            val tildeOffset = if (path.startsWith("~")) 1 else 0
            if (tildeOffset > 0) path = path.substring(1)
            if (!path.contains('.')) continue
            if (!path[0].isLowerCase() && path[0] != '_') continue
            out.add(matcher.start(group) + tildeOffset)
        }
    }

    private fun highlightRefs(
        pattern: Pattern,
        content: String,
        group: Int,
        contentOffset: Int,
        elementOffset: Int,
        holder: AnnotationHolder,
    ) {
        val matcher = pattern.matcher(content)
        while (matcher.find()) {
            var path = matcher.group(group) ?: continue
            val tildeOffset = if (path.startsWith("~")) 1 else 0
            if (tildeOffset > 0) path = path.substring(1)
            if (!path.contains('.')) continue
            if (!path[0].isLowerCase() && path[0] != '_') continue

            val pathStart = matcher.start(group) + tildeOffset

            // Highlight each segment as an identifier.
            val segments = path.split('.')
            var pos = pathStart
            for (segment in segments) {
                val absStart = elementOffset + contentOffset + pos
                val absEnd = absStart + segment.length
                val range = TextRange(absStart, absEnd)

                holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                    .range(range)
                    .textAttributes(DefaultLanguageHighlighterColors.IDENTIFIER)
                    .create()

                pos += segment.length + 1 // +1 for dot
            }

            // Highlight dots as punctuation.
            pos = pathStart
            for (i in 0 until segments.size - 1) {
                pos += segments[i].length
                val dotStart = elementOffset + contentOffset + pos
                holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                    .range(TextRange(dotStart, dotStart + 1))
                    .textAttributes(DefaultLanguageHighlighterColors.DOT)
                    .create()
                pos += 1
            }
        }
    }

    private fun highlightNativeRefs(
        content: String,
        contentOffset: Int,
        elementOffset: Int,
        existingOffsets: Set<Int>,
        holder: AnnotationHolder,
    ) {
        val matcher = DOXR_NATIVE.matcher(content)
        while (matcher.find()) {
            val start = matcher.start()

            // Skip if preceded by \ (escaped), ] (MkDocs second bracket),
            // or word char (subscript like AbstractBase[int]).
            if (start > 0) {
                val prev = content[start - 1]
                if (prev == '\\' || prev == ']' || prev.isLetterOrDigit() || prev == '_') continue
            }

            // Skip if followed by [ (MkDocs [path][] first part).
            val end = matcher.end()
            if (end < content.length && content[end] == '[') continue

            val path = matcher.group(1) ?: continue
            val pathStart = matcher.start(1)

            if (existingOffsets.contains(pathStart)) continue

            if (path.contains('.')) {
                // FQ — must start with lowercase/underscore.
                if (!path[0].isLowerCase() && path[0] != '_') continue

                // Highlight each segment as identifier, dots as punctuation.
                val segments = path.split('.')
                var pos = pathStart
                for (segment in segments) {
                    val absStart = elementOffset + contentOffset + pos
                    val absEnd = absStart + segment.length
                    holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                        .range(TextRange(absStart, absEnd))
                        .textAttributes(DefaultLanguageHighlighterColors.IDENTIFIER)
                        .create()
                    pos += segment.length + 1
                }
                pos = pathStart
                for (i in 0 until segments.size - 1) {
                    pos += segments[i].length
                    val dotStart = elementOffset + contentOffset + pos
                    holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                        .range(TextRange(dotStart, dotStart + 1))
                        .textAttributes(DefaultLanguageHighlighterColors.DOT)
                        .create()
                    pos += 1
                }
            } else {
                // Short name — single identifier highlight.
                val absStart = elementOffset + contentOffset + pathStart
                val absEnd = absStart + path.length
                holder.newSilentAnnotation(HighlightSeverity.INFORMATION)
                    .range(TextRange(absStart, absEnd))
                    .textAttributes(DefaultLanguageHighlighterColors.IDENTIFIER)
                    .create()
            }
        }
    }
}
