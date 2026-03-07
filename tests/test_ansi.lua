local MiniTest = require("mini.test")
local expect = MiniTest.expect

local T = MiniTest.new_set()

local jj = require("kenjutu.jj")
local parse_ansi_line = jj._test.parse_ansi_line
local ansi_256_to_hex = jj._test.ansi_256_to_hex
local strip_ansi = jj._test.strip_ansi

-- ansi_256_to_hex -------------------------------------------------------------

T["ansi_256_to_hex"] = MiniTest.new_set()

T["ansi_256_to_hex"]["standard colors 0-7"] = function()
  expect.equality(ansi_256_to_hex(0), "#000000")
  expect.equality(ansi_256_to_hex(1), "#800000")
  expect.equality(ansi_256_to_hex(7), "#c0c0c0")
end

T["ansi_256_to_hex"]["bright colors 8-15"] = function()
  expect.equality(ansi_256_to_hex(8), "#808080")
  expect.equality(ansi_256_to_hex(9), "#ff0000")
  expect.equality(ansi_256_to_hex(15), "#ffffff")
end

T["ansi_256_to_hex"]["6x6x6 color cube"] = function()
  -- Index 16 = rgb(0,0,0)
  expect.equality(ansi_256_to_hex(16), "#000000")
  -- Index 196 = rgb(5,0,0) => 255,0,0
  expect.equality(ansi_256_to_hex(196), "#ff0000")
  -- Index 21 = rgb(0,0,5) => 0,0,255
  expect.equality(ansi_256_to_hex(21), "#0000ff")
end

T["ansi_256_to_hex"]["grayscale ramp"] = function()
  -- Index 232 = level 8
  expect.equality(ansi_256_to_hex(232), "#080808")
  -- Index 255 = level 8 + 23*10 = 238
  expect.equality(ansi_256_to_hex(255), "#eeeeee")
end

-- strip_ansi ------------------------------------------------------------------

T["strip_ansi"] = MiniTest.new_set()

T["strip_ansi"]["removes escape codes"] = function()
  expect.equality(strip_ansi("\x1b[31mhello\x1b[0m"), "hello")
end

T["strip_ansi"]["passes through plain text"] = function()
  expect.equality(strip_ansi("no codes here"), "no codes here")
end

T["strip_ansi"]["handles multiple sequences"] = function()
  expect.equality(strip_ansi("\x1b[1m\x1b[31mbold red\x1b[0m"), "bold red")
end

-- parse_ansi_line -------------------------------------------------------------

T["parse_ansi_line"] = MiniTest.new_set()

T["parse_ansi_line"]["plain text returns unchanged with no highlights"] = function()
  local plain, highlights = parse_ansi_line("hello world")
  expect.equality(plain, "hello world")
  expect.equality(#highlights, 0)
end

T["parse_ansi_line"]["standard foreground color"] = function()
  local plain, highlights = parse_ansi_line("\x1b[31mred\x1b[0m")
  expect.equality(plain, "red")
  expect.equality(#highlights, 1)
  expect.equality(highlights[1].col_start, 0)
  expect.equality(highlights[1].col_end, 3)
end

T["parse_ansi_line"]["bright foreground color"] = function()
  local plain, highlights = parse_ansi_line("\x1b[91mbright red\x1b[0m")
  expect.equality(plain, "bright red")
  expect.equality(#highlights, 1)
end

T["parse_ansi_line"]["256-color foreground"] = function()
  local plain, highlights = parse_ansi_line("\x1b[38;5;196mtext\x1b[0m")
  expect.equality(plain, "text")
  expect.equality(#highlights, 1)
end

T["parse_ansi_line"]["24-bit RGB foreground"] = function()
  local plain, highlights = parse_ansi_line("\x1b[38;2;255;128;0mtext\x1b[0m")
  expect.equality(plain, "text")
  expect.equality(#highlights, 1)
end

T["parse_ansi_line"]["background colors"] = function()
  local plain, highlights = parse_ansi_line("\x1b[41mtext\x1b[0m")
  expect.equality(plain, "text")
  expect.equality(#highlights, 1)
end

T["parse_ansi_line"]["bold style"] = function()
  local plain, highlights = parse_ansi_line("\x1b[1mbold\x1b[22mnormal")
  expect.equality(plain, "boldnormal")
  expect.equality(#highlights, 1)
  expect.equality(highlights[1].col_start, 0)
  expect.equality(highlights[1].col_end, 4)
end

T["parse_ansi_line"]["nested styles in one sequence"] = function()
  local plain, highlights = parse_ansi_line("\x1b[1;31mbold red\x1b[0m")
  expect.equality(plain, "bold red")
  expect.equality(#highlights, 1)
end

T["parse_ansi_line"]["multiple styled segments"] = function()
  local plain, highlights = parse_ansi_line("\x1b[31mred\x1b[0m plain \x1b[32mgreen\x1b[0m")
  expect.equality(plain, "red plain green")
  expect.equality(#highlights, 2)
  -- First span: "red" at bytes 0-3
  expect.equality(highlights[1].col_start, 0)
  expect.equality(highlights[1].col_end, 3)
  -- Second span: "green" at bytes 10-15
  expect.equality(highlights[2].col_start, 10)
  expect.equality(highlights[2].col_end, 15)
end

T["parse_ansi_line"]["reset code clears all styles"] = function()
  local plain, highlights = parse_ansi_line("\x1b[1;31mbold red\x1b[0m after")
  expect.equality(plain, "bold red after")
  -- Only "bold red" should be highlighted, " after" should have no highlight
  expect.equality(#highlights, 1)
  expect.equality(highlights[1].col_end, 8)
end

T["parse_ansi_line"]["correct byte offsets with multi-byte text before"] = function()
  -- Text before ANSI, then colored text
  local plain, highlights = parse_ansi_line("prefix \x1b[31mred\x1b[0m")
  expect.equality(plain, "prefix red")
  expect.equality(#highlights, 1)
  expect.equality(highlights[1].col_start, 7)
  expect.equality(highlights[1].col_end, 10)
end

T["parse_ansi_line"]["malformed sequence without m terminator"] = function()
  local plain, _ = parse_ansi_line("before\x1b[31")
  -- Malformed ESC should be included as-is
  expect.equality(plain, "before\x1b[31")
end

-- parse_log_line (\\x01 marker extraction) ------------------------------------

local parse_log_line = jj._test.parse_log_line

T["parse_log_line"] = MiniTest.new_set()

T["parse_log_line"]["splits at marker boundary"] = function()
  local raw = "\x1b[31mheader text\x1b[0m\x01abc123\x00def456"
  local plain, highlights, commit = parse_log_line(raw)
  expect.equality(plain, "header text")
  expect.equality(#highlights, 1)
  expect.no_equality(commit, nil)
  expect.equality(commit.change_id, "abc123")
  expect.equality(commit.commit_id, "def456")
end

T["parse_log_line"]["strips ANSI from data portion"] = function()
  local raw = "display\x01\x1b[32mchange_id\x1b[0m\x00\x1b[33mcommit_id\x1b[0m"
  local plain, _, commit = parse_log_line(raw)
  expect.equality(plain, "display")
  expect.equality(commit.change_id, "change_id")
  expect.equality(commit.commit_id, "commit_id")
end

T["parse_log_line"]["handles plain text without marker"] = function()
  local plain, highlights, commit = parse_log_line("  description line")
  expect.equality(plain, "  description line")
  expect.equality(#highlights, 0)
  expect.equality(commit, nil)
end

T["parse_log_line"]["returns nil for blank lines"] = function()
  local plain, highlights, commit = parse_log_line("   ")
  expect.equality(plain, nil)
  expect.equality(highlights, nil)
  expect.equality(commit, nil)
end

return T
