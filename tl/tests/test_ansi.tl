local record TestModule
   eq: function(any, any, string)
   neq: function(any, any, string)
   ok: function(any, string)
   throws: function(function())
   debug: function(...: any)
   run_case: function(string, function())
end

local t = require("tests.test") as TestModule

local record JjTest
   parse_ansi_line: function(string): string, {Highlight}
   ansi_256_to_hex: function(integer): string
   strip_ansi: function(string): string
end

local record Highlight
   col_start: integer
   col_end: integer
end

local jj = require("kenjutu.jj") as any
local _test = jj._test as JjTest
local parse_ansi_line = _test.parse_ansi_line
local ansi_256_to_hex = _test.ansi_256_to_hex
local strip_ansi = _test.strip_ansi

t.run_case("ansi_256_to_hex standard colors 0-7", function()
  t.eq(ansi_256_to_hex(0), "#000000")
  t.eq(ansi_256_to_hex(1), "#800000")
  t.eq(ansi_256_to_hex(7), "#c0c0c0")
end)

t.run_case("ansi_256_to_hex bright colors 8-15", function()
  t.eq(ansi_256_to_hex(8), "#808080")
  t.eq(ansi_256_to_hex(9), "#ff0000")
  t.eq(ansi_256_to_hex(15), "#ffffff")
end)

t.run_case("ansi_256_to_hex 6x6x6 color cube", function()
  t.eq(ansi_256_to_hex(16), "#000000")
  t.eq(ansi_256_to_hex(196), "#ff0000")
  t.eq(ansi_256_to_hex(21), "#0000ff")
end)

t.run_case("ansi_256_to_hex grayscale ramp", function()
  t.eq(ansi_256_to_hex(232), "#080808")
  t.eq(ansi_256_to_hex(255), "#eeeeee")
end)

t.run_case("strip_ansi removes escape codes", function()
  t.eq(strip_ansi("\x1b[31mhello\x1b[0m"), "hello")
end)

t.run_case("strip_ansi passes through plain text", function()
  t.eq(strip_ansi("no codes here"), "no codes here")
end)

t.run_case("strip_ansi handles multiple sequences", function()
  t.eq(strip_ansi("\x1b[1m\x1b[31mbold red\x1b[0m"), "bold red")
end)

t.run_case("parse_ansi_line plain text returns unchanged with no highlights", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("hello world")
  t.eq(plain, "hello world")
  t.eq(#highlights, 0)
end)

t.run_case("parse_ansi_line standard foreground color", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[31mred\x1b[0m")
  t.eq(plain, "red")
  t.eq(#highlights, 1)
  t.eq(highlights[1].col_start, 0)
  t.eq(highlights[1].col_end, 3)
end)

t.run_case("parse_ansi_line bright foreground color", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[91mbright red\x1b[0m")
  t.eq(plain, "bright red")
  t.eq(#highlights, 1)
end)

t.run_case("parse_ansi_line 256-color foreground", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[38;5;196mtext\x1b[0m")
  t.eq(plain, "text")
  t.eq(#highlights, 1)
end)

t.run_case("parse_ansi_line 24-bit RGB foreground", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[38;2;255;128;0mtext\x1b[0m")
  t.eq(plain, "text")
  t.eq(#highlights, 1)
end)

t.run_case("parse_ansi_line background colors", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[41mtext\x1b[0m")
  t.eq(plain, "text")
  t.eq(#highlights, 1)
end)

t.run_case("parse_ansi_line bold style", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[1mbold\x1b[22mnormal")
  t.eq(plain, "boldnormal")
  t.eq(#highlights, 1)
  t.eq(highlights[1].col_start, 0)
  t.eq(highlights[1].col_end, 4)
end)

t.run_case("parse_ansi_line nested styles in one sequence", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[1;31mbold red\x1b[0m")
  t.eq(plain, "bold red")
  t.eq(#highlights, 1)
end)

t.run_case("parse_ansi_line multiple styled segments", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[31mred\x1b[0m plain \x1b[32mgreen\x1b[0m")
  t.eq(plain, "red plain green")
  t.eq(#highlights, 2)
  t.eq(highlights[1].col_start, 0)
  t.eq(highlights[1].col_end, 3)
  t.eq(highlights[2].col_start, 10)
  t.eq(highlights[2].col_end, 15)
end)

t.run_case("parse_ansi_line reset code clears all styles", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("\x1b[1;31mbold red\x1b[0m after")
  t.eq(plain, "bold red after")
  t.eq(#highlights, 1)
  t.eq(highlights[1].col_end, 8)
end)

t.run_case("parse_ansi_line correct byte offsets with text before", function()
  local plain: string, highlights: {Highlight} = parse_ansi_line("prefix \x1b[31mred\x1b[0m")
  t.eq(plain, "prefix red")
  t.eq(#highlights, 1)
  t.eq(highlights[1].col_start, 7)
  t.eq(highlights[1].col_end, 10)
end)

t.run_case("parse_ansi_line malformed sequence without m terminator", function()
  local plain: string, _: {Highlight} = parse_ansi_line("before\x1b[31")
  t.eq(plain, "before\x1b[31")
end)
