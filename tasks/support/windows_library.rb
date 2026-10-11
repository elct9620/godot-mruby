# frozen_string_literal: true

# Reads what a Windows library (a PE file) needs at load time, from its import
# table, so any host can check a DLL without Windows' tools. Backs Addon.
module WindowsLibrary
  PE32_PLUS = 0x20b
  SECTION_SIZE = 40
  DESCRIPTOR_SIZE = 20

  module_function

  # The DLLs the library imports, by the names its import table gives them.
  def read_imports(path)
    image = File.binread(path)
    header = image.unpack1("V", offset: 0x3c)
    raise "#{path} is not a PE file" unless image.byteslice(header, 4) == "PE\0\0"

    sections = read_sections(image, header)
    table = to_offset(sections, read_import_rva(image, header))
    return [] unless table

    read_name_rvas(image, table).map do |rva|
      offset = to_offset(sections, rva)
      image.byteslice(offset, image.index("\0", offset) - offset)
    end
  end

  # The name RVA of each import descriptor, up to the all-zero one that ends
  # the table.
  def read_name_rvas(image, table)
    (0..).lazy
         .map { |index| image.unpack1("V", offset: table + (index * DESCRIPTOR_SIZE) + 12) }
         .take_while(&:nonzero?)
      .to_a
  end

  # The import table's RVA: the second data directory, whose place follows
  # the optional header's PE32 or PE32+ layout.
  def read_import_rva(image, header)
    optional = header + 24
    directories = image.unpack1("v", offset: optional) == PE32_PLUS ? optional + 112 : optional + 96
    image.unpack1("V", offset: directories + 8)
  end

  # Each section as [RVA, size in memory, offset in the file].
  def read_sections(image, header)
    count = image.unpack1("v", offset: header + 6)
    first = header + 24 + image.unpack1("v", offset: header + 20)
    Array.new(count) do |index|
      size, rva, raw_size, raw_offset = image.unpack("V4", offset: first + (index * SECTION_SIZE) + 8)
      [rva, [size, raw_size].max, raw_offset]
    end
  end

  def to_offset(sections, rva)
    return nil if rva.zero?

    rva_start, _, raw_offset = sections.find { |start, size, _| rva >= start && rva < start + size }
    raw_offset && (rva - rva_start + raw_offset)
  end
end
