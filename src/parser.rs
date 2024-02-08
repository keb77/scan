mod bt;
mod fsm;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::io::BufRead;
use std::path::PathBuf;
use std::str;
use std::str::Utf8Error;

use log::{error, info, trace, warn};
use quick_xml::events::attributes::{AttrError, Attribute};
use quick_xml::{events, Error as XmlError};
use quick_xml::{events::Event, Reader};

use crate::model::{
    ChannelSystem, ChannelSystemBuilder, CsAction, CsError, CsExpr, CsFormula, CsLocation, CsVar,
    PgId, VarType,
};

#[derive(Debug)]
pub enum ParserErrorType {
    Reader(XmlError),
    UnknownEvent(Event<'static>),
    Attr(AttrError),
    UnknownKey(String),
    Utf8(Utf8Error),
    Cs(CsError),
    UnexpectedEndTag(String),
    MissingLocation,
    UnknownVar(String),
    MissingExpr,
    UnexpectedStartTag(String),
    MissingAttr(String),
}

impl fmt::Display for ParserErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParserErrorType::UnknownEvent(_) => write!(f, "self:#?"),
            ParserErrorType::Attr(_) => write!(f, "self:#?"),
            ParserErrorType::Reader(err) => err.fmt(f),
            ParserErrorType::Utf8(err) => err.fmt(f),
            ParserErrorType::UnknownKey(_) => write!(f, "self:#?"),
            ParserErrorType::Cs(err) => err.fmt(f),
            ParserErrorType::UnexpectedStartTag(_) => todo!(),
            ParserErrorType::UnexpectedEndTag(_) => write!(f, "self:#?"),
            ParserErrorType::MissingLocation => todo!(),
            ParserErrorType::UnknownVar(_) => todo!(),
            ParserErrorType::MissingExpr => todo!(),
            ParserErrorType::MissingAttr(_) => todo!(),
        }
    }
}

impl Error for ParserErrorType {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParserErrorType::Reader(err) => Some(err),
            ParserErrorType::Utf8(err) => Some(err),
            ParserErrorType::Cs(err) => Some(err),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct ParserError(pub(crate) usize, pub(crate) ParserErrorType);

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let byte = self.0;
        let err = &self.1;
        // Currently quick_xml only supports Reader byte position.
        // See https://github.com/tafia/quick-xml/issues/109
        write!(f, "parser error at byte {byte}: {err}")
    }
}

impl Error for ParserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.1.source()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConvinceTag {
    Specification,
    Model,
    Properties,
    Scxml,
    ComponentList,
    BlackBoard,
    SkillList,
    BtToSkillInterface,
    Bt,
    Component,
    SkillDeclaration,
    SkillDefinition,
    StructList,
    Enumeration,
    Service,
    Struct,
    StructData,
    Enum,
    Function,
}

impl From<ConvinceTag> for &str {
    fn from(value: ConvinceTag) -> Self {
        match value {
            ConvinceTag::Specification => "specification",
            ConvinceTag::Model => "model",
            ConvinceTag::Properties => "properties",
            ConvinceTag::Scxml => "scxml",
            ConvinceTag::ComponentList => "componentList",
            ConvinceTag::BlackBoard => "blackBoard",
            ConvinceTag::SkillList => "skillList",
            ConvinceTag::BtToSkillInterface => "btBoSkillInterface",
            ConvinceTag::Bt => "bt",
            ConvinceTag::Component => "component",
            ConvinceTag::SkillDeclaration => "skillDeclaration",
            ConvinceTag::SkillDefinition => "skillDefinition",
            ConvinceTag::StructList => "stuctList",
            ConvinceTag::Enumeration => "enumeration",
            ConvinceTag::Service => "service",
            ConvinceTag::Struct => "struct",
            ConvinceTag::StructData => "struct_data",
            ConvinceTag::Enum => "enum",
            ConvinceTag::Function => "function",
        }
    }
}

#[derive(Debug)]
pub struct Parser {
    model: ChannelSystemBuilder,
    skills: HashMap<String, PgId>,
    states: HashMap<String, CsLocation>,
    events: HashMap<String, CsAction>,
    vars: HashMap<String, CsVar>,
}

impl Parser {
    const SPECIFICATION: &'static str = "specification";
    const MODEL: &'static str = "model";
    const PROPERTIES: &'static str = "properties";
    const COMPONENT_LIST: &'static str = "componentList";
    const BLACK_BOARD: &'static str = "blackBoard";
    const SKILL_LIST: &'static str = "skillList";
    const BT_TO_SKILL_INTERFACE: &'static str = "btToSkillInterface";
    const BT: &'static str = "bt";
    const COMPONENT: &'static str = "component";
    const SKILL_DECLARATION: &'static str = "skillDeclaration";
    const SKILL_DEFINITION: &'static str = "skillDefinition";
    const FILE: &'static str = "file";
    const STATE: &'static str = "state";
    const SCXML: &'static str = "scxml";
    const INITIAL: &'static str = "initial";
    const ID: &'static str = "id";
    const MOC: &'static str = "moc";
    const PATH: &'static str = "path";
    const FSM: &'static str = "fsm";
    const INTERFACE: &'static str = "interface";
    const VERSION: &'static str = "version";
    const NAME: &'static str = "name";
    const XMLNS: &'static str = "xmlns";
    const DATAMODEL: &'static str = "datamodel";
    const DATA: &'static str = "data";
    const TYPE: &'static str = "type";
    const BOOL: &'static str = "bool";
    const INT: &'static str = "int";
    const UNIT: &'static str = "unit";
    const BINDING: &'static str = "binding";
    const TRANSITION: &'static str = "transition";
    const TARGET: &'static str = "target";
    const EVENT: &'static str = "event";
    const ON_ENTRY: &'static str = "onentry";
    const ON_EXIT: &'static str = "onexit";
    const NULL: &'static str = "NULL";
    const SCRIPT: &'static str = "script";
    const ASSIGN: &'static str = "assign";
    const LOCATION: &'static str = "location";
    const EXPR: &'static str = "expr";
    const RAISE: &'static str = "raise";
    const STRUCT_LIST: &'static str = "structList";
    const ENUMERATION: &'static str = "enumeration";
    const SERVICE: &'static str = "service";
    const STRUCT: &'static str = "struct";
    const STRUCT_DATA: &'static str = "structData";
    const FIELD_ID: &'static str = "fieldId";
    const ENUM: &'static str = "enum";
    const FUNCTION: &'static str = "function";

    pub fn parse<R: BufRead>(reader: &mut Reader<R>) -> anyhow::Result<ChannelSystem> {
        let mut parser = Self {
            model: ChannelSystemBuilder::default(),
            skills: HashMap::default(),
            states: HashMap::default(),
            events: HashMap::default(),
            vars: HashMap::default(),
        };
        let mut buf = Vec::new();
        let mut stack = Vec::new();
        info!("begin parsing");
        loop {
            match reader.read_event_into(&mut buf)? {
                Event::Start(tag) => {
                    let tag_name: String = String::from_utf8(tag.name().as_ref().to_vec())?;
                    trace!("'{tag_name}' open tag");
                    match tag_name.as_str() {
                        Self::SPECIFICATION if stack.is_empty() => {
                            stack.push(ConvinceTag::Specification);
                        }
                        Self::MODEL
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::Specification) =>
                        {
                            stack.push(ConvinceTag::Model);
                        }
                        Self::PROPERTIES
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::Specification) =>
                        {
                            stack.push(ConvinceTag::Properties);
                        }
                        Self::COMPONENT_LIST
                            if stack.last().is_some_and(|tag| *tag == ConvinceTag::Model) =>
                        {
                            stack.push(ConvinceTag::ComponentList);
                        }
                        Self::BLACK_BOARD
                            if stack.last().is_some_and(|tag| *tag == ConvinceTag::Model) =>
                        {
                            stack.push(ConvinceTag::BlackBoard);
                        }
                        Self::SKILL_LIST
                            if stack.last().is_some_and(|tag| *tag == ConvinceTag::Model) =>
                        {
                            stack.push(ConvinceTag::SkillList);
                        }
                        Self::SKILL_DECLARATION
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::SkillList) =>
                        {
                            parser.parse_skill_declaration(tag, reader)?;
                            stack.push(ConvinceTag::SkillDeclaration);
                        }
                        Self::SKILL_DEFINITION
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::SkillDeclaration) =>
                        {
                            parser.parse_skill_definition(tag, reader)?;
                            stack.push(ConvinceTag::SkillDefinition);
                        }
                        Self::BT_TO_SKILL_INTERFACE
                            if stack.last().is_some_and(|tag| *tag == ConvinceTag::Model) =>
                        {
                            stack.push(ConvinceTag::BtToSkillInterface);
                        }
                        Self::BT if stack.last().is_some_and(|tag| *tag == ConvinceTag::Model) => {
                            parser.parse_bt(tag)?;
                            stack.push(ConvinceTag::Bt);
                        }
                        Self::COMPONENT
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::ComponentList) =>
                        {
                            parser.parse_component(tag)?;
                            stack.push(ConvinceTag::Component);
                        }
                        Self::STRUCT_LIST
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::Component) =>
                        {
                            stack.push(ConvinceTag::StructList);
                        }
                        Self::ENUMERATION
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::Component) =>
                        {
                            stack.push(ConvinceTag::Enumeration);
                        }
                        Self::SERVICE
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::Component) =>
                        {
                            stack.push(ConvinceTag::Service);
                        }
                        Self::STRUCT
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::StructList) =>
                        {
                            parser.parse_struct(tag)?;
                            stack.push(ConvinceTag::Struct);
                        }
                        Self::STRUCT_DATA
                            if stack.last().is_some_and(|tag| *tag == ConvinceTag::Struct) =>
                        {
                            parser.parse_structdata(tag)?;
                            stack.push(ConvinceTag::StructData);
                        }
                        Self::ENUM
                            if stack
                                .last()
                                .is_some_and(|tag| *tag == ConvinceTag::Enumeration) =>
                        {
                            stack.push(ConvinceTag::Enum);
                        }
                        Self::FUNCTION
                            if stack.last().is_some_and(|tag| *tag == ConvinceTag::Service) =>
                        {
                            stack.push(ConvinceTag::Function);
                        }
                        // Unknown tag: skip till maching end tag
                        tag_name => {
                            warn!("unknown or unexpected tag {tag_name}, skipping");
                            reader.read_to_end_into(tag.to_end().into_owned().name(), &mut buf)?;
                        }
                    }
                }
                // exits the loop when reaching end of file
                Event::Eof => {
                    info!("parsing completed");
                    let model = parser.model.build();
                    return Ok(model);
                }
                Event::End(tag) => {
                    let name = tag.name();
                    let name = str::from_utf8(name.as_ref())?;
                    if stack.pop().is_some_and(|tag| <&str>::from(tag) == name) {
                        trace!("'{name}' end tag");
                    } else {
                        error!("unexpected end tag {name}");
                        return Err(anyhow::Error::new(ParserError(
                            reader.buffer_position(),
                            ParserErrorType::UnexpectedEndTag(name.to_string()),
                        )));
                    }
                }
                Event::Empty(tag) => warn!("skipping empty tag"),
                Event::Text(_) => warn!("skipping text"),
                Event::Comment(_) => warn!("skipping comment"),
                Event::CData(_) => todo!(),
                Event::Decl(tag) => parser.parse_xml_declaration(tag)?,
                Event::PI(_) => todo!(),
                Event::DocType(_) => todo!(),
            }
            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        }
    }

    fn parse_xml_declaration(&self, tag: events::BytesDecl<'_>) -> Result<(), ParserError> {
        // TODO: parse attributes
        Ok(())
    }

    fn parse_skill_declaration<R: BufRead>(
        &mut self,
        tag: events::BytesStart,
        reader: &Reader<R>,
    ) -> anyhow::Result<()> {
        for attr in tag
            .attributes()
            .into_iter()
            .collect::<Result<Vec<Attribute>, AttrError>>()?
        {
            match str::from_utf8(attr.key.as_ref())? {
                Self::ID => {
                    let skill_id = str::from_utf8(attr.value.as_ref())?;
                    if !self.skills.contains_key(skill_id) {
                        let pg_id = self.model.new_program_graph();
                        self.skills.insert(skill_id.to_string(), pg_id);
                    }
                }
                Self::INTERFACE => warn!("ignoring interface for now"),
                key => {
                    error!(
                        "found unknown attribute {key} in {}",
                        Self::SKILL_DECLARATION
                    );
                    return Err(anyhow::Error::new(ParserError(
                        reader.buffer_position(),
                        ParserErrorType::UnknownKey(key.to_owned()),
                    )));
                }
            }
        }
        Ok(())
    }

    fn parse_skill_definition<R: BufRead>(
        &mut self,
        tag: events::BytesStart,
        reader: &Reader<R>,
    ) -> anyhow::Result<()> {
        let mut moc = None;
        let mut path = None;
        for attr in tag
            .attributes()
            .into_iter()
            .collect::<Result<Vec<Attribute>, AttrError>>()?
        {
            match str::from_utf8(attr.key.as_ref())? {
                Self::TYPE => {
                    todo!()
                }
                Self::MOC => moc = Some(String::from_utf8(attr.value.into_owned())?),
                Self::PATH => {
                    path = Some(PathBuf::try_from(String::from_utf8(
                        attr.value.into_owned(),
                    )?)?)
                }
                key => {
                    error!(
                        "found unknown attribute {key} in {}",
                        Self::SKILL_DECLARATION
                    );
                    return Err(anyhow::Error::new(ParserError(
                        reader.buffer_position(),
                        ParserErrorType::UnknownKey(key.to_owned()),
                    )));
                }
            }
        }
        let path = path.ok_or(ParserError(
            reader.buffer_position(),
            ParserErrorType::MissingAttr(Self::PATH.to_string()),
        ))?;
        info!("creating reader from file {path:?}");
        let mut reader = Reader::from_file(path)?;
        match moc.as_deref() {
            Some(Self::FSM) => {
                self.parse_skill(&mut reader)?;
            }
            Some(Self::BT) => {
                self.parse_skill(&mut reader)?;
            }
            Some(_) => {
                error!("unrecognized moc");
            }
            None => {
                error!("missing attribute moc");
            }
        }
        Ok(())
    }

    fn parse_properties<R: BufRead>(
        &mut self,
        tag: &events::BytesStart,
        reader: &mut Reader<R>,
    ) -> Result<(), ParserError> {
        todo!()
    }

    fn parse_datamodel<R: BufRead>(
        &mut self,
        _tag: &events::BytesStart,
        reader: &mut Reader<R>,
        pg_id: PgId,
    ) -> Result<(), ParserError> {
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf).map_err(|err| {
                ParserError(reader.buffer_position(), ParserErrorType::Reader(err))
            })? {
                Event::Empty(tag) => match str::from_utf8(tag.name().as_ref()).map_err(|err| {
                    ParserError(reader.buffer_position(), ParserErrorType::Utf8(err))
                })? {
                    Self::DATA => self.parse_data(reader, &tag, pg_id)?,
                    tag_name => warn!("unknown empty tag {tag_name}, skipping"),
                },
                Event::Start(tag) => match str::from_utf8(tag.name().as_ref()).map_err(|err| {
                    ParserError(reader.buffer_position(), ParserErrorType::Utf8(err))
                })? {
                    // Unknown tag: skip till maching end tag
                    tag_name => {
                        warn!("unknown tag {tag_name}, skipping");
                        reader
                            .read_to_end_into(tag.to_end().into_owned().name(), &mut buf)
                            .map_err(|err| {
                                ParserError(reader.buffer_position(), ParserErrorType::Reader(err))
                            })?;
                    }
                },
                Event::End(tag) => match str::from_utf8(tag.name().as_ref()).map_err(|err| {
                    ParserError(reader.buffer_position(), ParserErrorType::Utf8(err))
                })? {
                    Self::DATAMODEL => return Ok(()),
                    name => {
                        error!("unexpected end tag {name}");
                        return Err(ParserError(
                            reader.buffer_position(),
                            ParserErrorType::UnexpectedEndTag(name.to_string()),
                        ));
                    }
                },
                Event::Eof => todo!(),
                Event::Text(_) => warn!("skipping text"),
                Event::Comment(_) => warn!("skipping comment"),
                Event::CData(_) => todo!(),
                Event::Decl(_) => todo!(),
                Event::PI(_) => todo!(),
                Event::DocType(_) => todo!(),
            }
            // if we don't keep a borrow elsewhere, we can clear the buffer to keep memory usage low
            buf.clear();
        }
    }

    fn parse_data<R: BufRead>(
        &mut self,
        reader: &mut Reader<R>,
        tag: &events::BytesStart<'_>,
        pg_id: PgId,
    ) -> Result<(), ParserError> {
        let mut id = None;
        let mut var_type = VarType::Unit;
        // let mut value = None;
        for attr in tag.attributes() {
            let attr = attr
                .map_err(|err| ParserError(reader.buffer_position(), ParserErrorType::Attr(err)))?;
            match str::from_utf8(attr.key.as_ref())
                .map_err(|err| ParserError(reader.buffer_position(), ParserErrorType::Utf8(err)))?
            {
                Self::ID => {
                    id = Some(String::from_utf8(attr.value.to_vec()).map_err(|err| {
                        ParserError(
                            reader.buffer_position(),
                            ParserErrorType::Utf8(err.utf8_error()),
                        )
                    })?);
                }
                Self::TYPE => {
                    match str::from_utf8(attr.value.as_ref()).map_err(|err| {
                        ParserError(reader.buffer_position(), ParserErrorType::Utf8(err))
                    })? {
                        Self::BOOL => var_type = VarType::Boolean,
                        Self::INT => var_type = VarType::Integer,
                        Self::UNIT => var_type = VarType::Unit,
                        _ => error!("unknown data type, ignoring"),
                    }
                }
                name => warn!("unknown attribute {name}, ignoring"),
            }
        }
        if let Some(id) = id {
            let val_id = self
                .model
                .new_var(pg_id, var_type)
                .map_err(|err| ParserError(reader.buffer_position(), ParserErrorType::Cs(err)))?;
            self.vars.insert(id, val_id);
        } else {
            todo!()
        }
        Ok(())
    }

    // fn parse_script<R: BufRead>(
    //     &self,
    //     tag: events::BytesStart,
    //     reader: &mut Reader<R>,
    //     pg_id: PgId,
    // ) -> Result<(), ParserError> {
    //     todo!()
    // }

    fn parse_bt(
        &self,
        // reader: &mut Reader<R>,
        tag: events::BytesStart<'_>,
    ) -> anyhow::Result<()> {
        for attr in tag.attributes() {
            let attr = attr?;
            match str::from_utf8(attr.key.as_ref())? {
                Self::FILE => {
                    let file = str::from_utf8(attr.value.as_ref())?;
                    let file = PathBuf::try_from(file)?;
                    todo!()
                }
                name => error!("unknown attribute {name}, ignoring"),
            }
        }
        Ok(())
    }

    fn parse_component(&self, tag: events::BytesStart<'_>) -> anyhow::Result<()> {
        for attr in tag.attributes() {
            let attr = attr?;
            match str::from_utf8(attr.key.as_ref())? {
                Self::ID => {
                    let id = str::from_utf8(attr.value.as_ref())?;
                    todo!()
                }
                name => error!("unknown attribute {name}, ignoring"),
            }
        }
        Ok(())
    }

    fn parse_struct(&self, tag: events::BytesStart<'_>) -> anyhow::Result<()> {
        for attr in tag.attributes() {
            let attr = attr?;
            match str::from_utf8(attr.key.as_ref())? {
                Self::ID => {
                    let id = str::from_utf8(attr.value.as_ref())?;
                    todo!()
                }
                name => error!("unknown attribute {name}, ignoring"),
            }
        }
        Ok(())
    }

    fn parse_structdata(&self, tag: events::BytesStart<'_>) -> anyhow::Result<()> {
        for attr in tag.attributes() {
            let attr = attr?;
            match str::from_utf8(attr.key.as_ref())? {
                Self::FIELD_ID => {
                    let field_id = str::from_utf8(attr.value.as_ref())?;
                    let field_id = field_id.parse::<usize>()?;
                    todo!()
                }
                name => error!("unknown attribute {name}, ignoring"),
            }
        }
        Ok(())
    }
}
