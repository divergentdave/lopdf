use super::{Dictionary, Document, Object, ObjectId};

impl Document {
	/// Create new PDF document with version.
	pub fn with_version<S: Into<String>>(version: S) -> Document {
		let mut document = Self::new();
		document.version = version.into();
		document
	}

	/// Create an object ID.
	pub fn new_object_id(&mut self) -> ObjectId {
		self.max_id += 1;
		(self.max_id, 0)
	}

	/// Add PDF object into document's object list.
	pub fn add_object<T: Into<Object>>(&mut self, object: T) -> ObjectId {
		self.max_id += 1;
		let id = (self.max_id, 0);
		self.objects.insert(id, object.into());
		id
	}

	fn get_or_create_resources_mut(&mut self, page_id: ObjectId) -> Option<&mut Object> {
		let page = self.get_object_mut(page_id).and_then(|obj| obj.as_dict_mut()).unwrap();
		if page.has(b"Resources") {
			if let Some(_res_id) = page.get(b"Resources").and_then(|obj| obj.as_reference()) {
				// self.get_object_mut(res_id)
				None
			} else {
				page.get_mut(b"Resources")
			}
		} else {
			page.set("Resources", Dictionary::new());
			page.get_mut(b"Resources")
		}
	}

	pub fn get_or_create_resources(&mut self, page_id: ObjectId) -> Option<&mut Object> {
		let mut resources_id = None;
		{
			let page = self.get_object(page_id).and_then(|obj| obj.as_dict()).unwrap();
			if page.has(b"Resources") {
				resources_id = page.get(b"Resources").and_then(|obj| obj.as_reference());
			}
		}
		match resources_id {
			Some(res_id) => self.get_object_mut(res_id),
			None => self.get_or_create_resources_mut(page_id),
		}
	}

	pub fn add_xobject<N: Into<Vec<u8>>>(&mut self, page_id: ObjectId, xobject_name: N, xobject_id: ObjectId) {
		if let Some(resources) = self.get_or_create_resources(page_id).and_then(|obj| obj.as_dict_mut()) {
			if !resources.has(b"XObject") {
				resources.set("XObject", Dictionary::new());
			}
			let xobjects = resources.get_mut(b"XObject").and_then(|obj| obj.as_dict_mut()).unwrap();
			xobjects.set(xobject_name, Object::Reference(xobject_id));
		}
	}

	pub fn add_graphics_state<N: Into<Vec<u8>>>(&mut self, page_id: ObjectId, gs_name: N, gs_id: ObjectId) {
		if let Some(resources) = self.get_or_create_resources(page_id).and_then(|obj| obj.as_dict_mut()) {
			if !resources.has(b"ExtGState") {
				resources.set("ExtGState", Dictionary::new());
			}
			let states = resources.get_mut(b"ExtGState").and_then(|obj| obj.as_dict_mut()).unwrap();
			states.set(gs_name, Object::Reference(gs_id));
		}
	}
}

#[test]
fn create_document() {
	use super::content::*;
	use super::Stream;
	use time::now;

	let mut doc = Document::with_version("1.5");
	let info_id = doc.add_object(dictionary! {
		"Title" => Object::string_literal("Create PDF document example"),
		"Creator" => Object::string_literal("https://crates.io/crates/lopdf"),
		"CreationDate" => now(),
	});
	let pages_id = doc.new_object_id();
	let font_id = doc.add_object(dictionary! {
		"Type" => "Font",
		"Subtype" => "Type1",
		"BaseFont" => "Courier",
	});
	let resources_id = doc.add_object(dictionary! {
		"Font" => dictionary! {
			"F1" => font_id,
		},
	});
	let content = Content {
		operations: vec![
			Operation::new("BT", vec![]),
			Operation::new("Tf", vec!["F1".into(), 48.into()]),
			Operation::new("Td", vec![100.into(), 600.into()]),
			Operation::new("Tj", vec![Object::string_literal("Hello World!")]),
			Operation::new("ET", vec![]),
		],
	};
	let content_id = doc.add_object(Stream::new(dictionary!{}, content.encode().unwrap()));
	let page_id = doc.add_object(dictionary! {
		"Type" => "Page",
		"Parent" => pages_id,
		"Contents" => content_id,
	});
	let pages = dictionary! {
		"Type" => "Pages",
		"Kids" => vec![page_id.into()],
		"Count" => 1,
		"Resources" => resources_id,
		"MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
	};
	doc.objects.insert(pages_id, Object::Dictionary(pages));
	let catalog_id = doc.add_object(dictionary! {
		"Type" => "Catalog",
		"Pages" => pages_id,
	});
	doc.trailer.set("Root", catalog_id);
	doc.trailer.set("Info", info_id);
	doc.compress();

	doc.save("test_1_create.pdf").unwrap();

	// test load_from() and save_to()
	use std::fs::File;
	use std::io::Cursor;

	let in_file = File::open("test_1_create.pdf").unwrap();
	let mut in_doc = Document::load_from(in_file).unwrap();

	let out_buf = Vec::<u8>::new();
	let mut memory_cursor = Cursor::new(out_buf);
	in_doc.save_to(&mut memory_cursor).unwrap();
	assert!(!memory_cursor.get_ref().is_empty());
}
