local grammar_path = vim.fn.system("nix build .#tree-sitter-grammar --no-link --print-out-paths 2>/dev/null"):gsub("%s+$", "")
if vim.v.shell_error ~= 0 or grammar_path == "" then
  vim.notify("sol: failed to build tree-sitter grammar via nix", vim.log.levels.ERROR)
  return
end

vim.filetype.add({ extension = { sol = "sol" } })

local runtime_dir = vim.fn.stdpath("data") .. "/sol-treesitter"
vim.fn.mkdir(runtime_dir .. "/parser", "p")
vim.fn.mkdir(runtime_dir .. "/queries/sol", "p")

vim.uv.fs_copyfile(grammar_path .. "/parser", runtime_dir .. "/parser/sol.so")

local query_src = vim.fn.getcwd() .. "/tree-sitter/queries/highlights.scm"
vim.uv.fs_copyfile(query_src, runtime_dir .. "/queries/sol/highlights.scm")

vim.opt.runtimepath:prepend(runtime_dir)

vim.api.nvim_create_autocmd("FileType", {
  pattern = "sol",
  callback = function(ev)
    vim.treesitter.start(ev.buf, "sol")
  end,
})
