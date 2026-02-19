ROOT := $(shell cd .. && pwd)
RUNTIME := $(ROOT)/syntaqlite-runtime
DIALECT := $(ROOT)/syntaqlite

CC := cc
CXX := c++
CFLAGS := -Wall -Wextra -Wpedantic -std=c11 \
	-I$(RUNTIME)/include \
	-I$(DIALECT)/include \
	-I$(RUNTIME) \
	-I$(DIALECT)
CXXFLAGS := -Wall -Wextra -Wpedantic -std=c++17 \
	-I$(RUNTIME)/include \
	-I$(DIALECT)/include \
	-I$(RUNTIME) \
	-I$(DIALECT)

# C source files needed for the engine + dialect.
ENGINE_SRCS := \
	$(RUNTIME)/csrc/arena.c \
	$(RUNTIME)/csrc/tokenizer.c \
	$(RUNTIME)/csrc/parser.c \
	$(RUNTIME)/csrc/sqlite_tokenize.c \
	$(RUNTIME)/csrc/sqlite_keyword.c

LEMON_SRCS := $(RUNTIME)/csrc/sqlite_parse.c
LEMON_CFLAGS := \
	-DSYNTAQLITE_INLINE_PARSER_DATA_HEADER='"csrc/dialect_parse.h"' \
	-Wno-int-conversion \
	-Wno-void-pointer-to-int-cast \
	-Wno-unused-variable \
	-Wno-unused-parameter \
	-Wno-comment

DIALECT_SRCS := $(DIALECT)/csrc/dialect.c

# Build engine + dialect as a static lib so we don't recompile for each example.
ENGINE_OBJS := $(patsubst %.c,%.o,$(notdir $(ENGINE_SRCS)))
LEMON_OBJS := sqlite_parse.o
DIALECT_OBJS := dialect.o
ALL_OBJS := $(ENGINE_OBJS) $(LEMON_OBJS) $(DIALECT_OBJS)

.PHONY: all clean

all: select_columns select_columns_c

# Engine objects
arena.o: $(RUNTIME)/csrc/arena.c
	$(CC) $(CFLAGS) -c -o $@ $<
tokenizer.o: $(RUNTIME)/csrc/tokenizer.c
	$(CC) $(CFLAGS) -c -o $@ $<
parser.o: $(RUNTIME)/csrc/parser.c
	$(CC) $(CFLAGS) -c -o $@ $<
sqlite_tokenize.o: $(RUNTIME)/csrc/sqlite_tokenize.c
	$(CC) $(CFLAGS) -c -o $@ $<
sqlite_keyword.o: $(RUNTIME)/csrc/sqlite_keyword.c
	$(CC) $(CFLAGS) -c -o $@ $<

# Lemon parser (relaxed warnings)
sqlite_parse.o: $(RUNTIME)/csrc/sqlite_parse.c
	$(CC) $(CFLAGS) $(LEMON_CFLAGS) -c -o $@ $<

# Dialect glue
dialect.o: $(DIALECT)/csrc/dialect.c
	$(CC) $(CFLAGS) -c -o $@ $<

libsyntaqlite.a: $(ALL_OBJS)
	ar rcs $@ $^

select_columns: select_columns.cc libsyntaqlite.a
	$(CXX) $(CXXFLAGS) -o $@ $^

select_columns_c: select_columns.c libsyntaqlite.a
	$(CC) $(CFLAGS) -o $@ $^

clean:
	rm -f select_columns select_columns_c libsyntaqlite.a *.o
