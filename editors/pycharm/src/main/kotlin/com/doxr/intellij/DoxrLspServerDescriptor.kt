package com.doxr.intellij

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor

class DoxrLspServerDescriptor(project: Project) :
    ProjectWideLspServerDescriptor(project, "doxr") {

    override fun isSupportedFile(file: VirtualFile): Boolean = file.extension == "py"

    override fun createCommandLine(): GeneralCommandLine {
        // Looks for `doxr` on PATH. Users install via:
        //   cargo install --git https://github.com/scriptogre/doxr
        // or:
        //   uv tool install git+https://github.com/scriptogre/doxr
        return GeneralCommandLine("doxr", "lsp")
            .withWorkDirectory(project.basePath)
    }
}
