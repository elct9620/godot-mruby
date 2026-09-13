# The compiler warns at the block's end that this else is useless; it runs.
begin
  :body
else
  :useless
end
