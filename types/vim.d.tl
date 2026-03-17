global record vim

   record BufOption
      buftype: string
      filetype: string
      swapfile: boolean
      buflisted: boolean
      bufhidden: string
      modifiable: boolean
      modified: boolean
   end

   record WinOption
      cursorline: boolean
      number: boolean
      relativenumber: boolean
      signcolumn: string
      wrap: boolean
      winfixwidth: boolean
      diff: boolean
      foldenable: boolean
      foldmethod: string
      foldexpr: string
      foldtext: string
      foldcolumn: string
      foldlevel: integer
      fillchars: string
   end

   record api
      nvim_create_namespace: function(name: string): integer
      nvim_set_hl: function(ns: integer, name: string, def: {string:any})
      nvim_create_buf: function(listed: boolean, scratch: boolean): integer
      nvim_buf_set_lines: function(buf: integer, start: integer, end_: integer, strict: boolean, lines: {string})
      nvim_buf_get_lines: function(buf: integer, start: integer, end_: integer, strict: boolean): {string}
      nvim_buf_set_name: function(buf: integer, name: string)
      nvim_buf_get_name: function(buf: integer): string
      nvim_buf_set_extmark: function(buf: integer, ns: integer, line: integer, col: integer, opts: {string:any}): integer
      nvim_buf_clear_namespace: function(buf: integer, ns: integer, start: integer, end_: integer)
      nvim_buf_is_valid: function(buf: integer): boolean
      nvim_buf_delete: function(buf: integer, opts: {string:any})
      nvim_buf_line_count: function(buf: integer): integer
      nvim_buf_get_keymap: function(buf: integer, mode: string): {{string:any}}
      nvim_buf_get_extmarks: function(buf: integer, ns: integer, start: any, end_: any, opts: {string:any}): {{any}}
      nvim_win_set_buf: function(win: integer, buf: integer)
      nvim_win_get_buf: function(win: integer): integer
      nvim_win_is_valid: function(win: integer): boolean
      nvim_win_close: function(win: integer, force: boolean)
      nvim_win_set_cursor: function(win: integer, pos: {integer})
      nvim_win_get_cursor: function(win: integer): {integer}
      nvim_win_set_width: function(win: integer, width: integer)
      nvim_win_get_height: function(win: integer): integer
      nvim_win_get_position: function(win: integer): {integer}
      nvim_win_get_config: function(win: integer): {string:any}
      nvim_win_call: function(win: integer, fun: function(): any): any
      nvim_open_win: function(buf: integer, enter: boolean, config: {string:any}): integer
      nvim_set_current_win: function(win: integer)
      nvim_get_current_win: function(): integer
      nvim_get_current_buf: function(): integer
      nvim_set_current_buf: function(buf: integer)
      nvim_create_autocmd: function(event: any, opts: {string:any}): integer
      nvim_create_user_command: function(name: string, command: function(any), opts: {string:any})
      nvim_feedkeys: function(keys: string, mode: string, escape_csi: boolean)
      nvim_replace_termcodes: function(str: string, from_part: boolean, do_lt: boolean, special: boolean): string
      nvim_echo: function(chunks: {{any}}, history: boolean, opts: {string:any})
      nvim_list_tabpages: function(): {integer}
      nvim_tabpage_get_number: function(tab: integer): integer
      nvim_tabpage_list_wins: function(tab: integer): {integer}
   end

   bo: {integer: BufOption}
   wo: {integer: WinOption}
   b: {integer: {string:any}}

   record ORecord
      columns: integer
      lines: integer
   end
   o: ORecord

   record VRecord
      foldstart: integer
      foldend: integer
      foldlevel: integer
      shell_error: integer
   end
   v: VRecord

   record fn
      jobstart: function(cmd: {string}, opts: {string:any}): integer
      chansend: function(id: integer, data: string): integer
      jobstop: function(id: integer)
      json_encode: function(obj: any): string
      json_decode: function(str: string): any
      sign_define: function(name: string, dict: {string:any})
      sign_place: function(id: integer, group: string, name: string, buf: integer, opts: {string:any}): integer
      sign_unplace: function(group: string, opts: {string:any})
      sign_getplaced: function(buf: integer, opts: {string:any}): {{string:any}}
      getcwd: function(): string
      fnamemodify: function(path: string, mod: string): string
      mode: function(): string
      line: function(expr: string): integer
      getline: function(lnum: integer): string
      trim: function(s: string): string
      system: function(cmd: any): string
      bufnr: function(name: string): integer
      bufwinid: function(bufnr: integer): integer
      winlayout: function(): {any}
      executable: function(name: string): integer
      glob: function(pattern: string, nosuf: boolean, list: boolean): {string}
      mkdir: function(dir: string, flags: string)
      readfile: function(path: string): {string}
   end

   cmd: function(str: string)
   notify: function(msg: string, level: integer, opts: {string:any})
   schedule: function(fn: function())
   schedule_wrap: function(fn: function(any): any): function(any): any
   split: function(s: string, sep: string, opts: {string:any}): {string}
   trim: function(s: string): string
   deep_equal: function(a: any, b: any): boolean
   inspect: function(obj: any): string
   list_extend: function(dst: {any}, src: {any}): {any}
   startswith: function(s: string, prefix: string): boolean
   tbl_extend: function(behavior: string, ...: {string:any}): {string:any}
   wait: function(timeout: integer, fn: function(): boolean): boolean
   system: function(cmd: {string}, opts: {string:any}, on_exit: function(any)): any

   record keymap
      set: function(mode: any, lhs: string, rhs: any, opts: {string:any})
   end

   record filetype
      match: function(opts: {string:any}): string
   end

   record json
      decode: function(str: string): any
   end

   record UvUname
      sysname: string
      release: string
      version: string
      machine: string
   end

   record uv
      os_uname: function(): UvUname
      fs_stat: function(path: string): {string:any}
   end

   record LogLevels
      ERROR: integer
      WARN: integer
      INFO: integer
   end

   record log
      levels: LogLevels
   end

   NIL: any

   record OptRtp
      prepend: function(OptRtp, path: string)
   end

   record opt
      rtp: OptRtp
   end

end
